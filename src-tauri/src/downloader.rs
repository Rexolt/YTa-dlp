use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::bin;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::ytdlp::{
    args::{build_args, FILEPATH_PREFIX},
    progress::parse_line,
    DownloadOptions,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadStarted {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadLog {
    pub id: String,
    pub stream: &'static str, // "stdout" | "stderr"
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadFinished {
    pub id: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub canceled: bool,
    pub output_path: Option<String>,
}

pub const EVT_STARTED: &str = "download://started";
pub const EVT_PROGRESS: &str = "download://progress";
pub const EVT_LOG: &str = "download://log";
pub const EVT_FINISHED: &str = "download://finished";

#[derive(Debug)]
pub enum RunOutcome {
    Success,
    /// The video was downloaded and an output file exists, but yt-dlp
    /// returned a non-zero exit code (typically from a non-fatal
    /// post-processing failure like thumbnail embedding on ffmpeg-free).
    PartialSuccess { warning: String },
    Failed { code: i32, stderr_tail: String },
    Canceled,
}

/// One attempt at running yt-dlp for a given download id.
/// Streams progress + logs as Tauri events, persists progress into the DB,
/// and respects a cancellation token. Caller (queue manager) handles retries
/// and final status reconciliation.
pub async fn run_ytdlp_once(
    app: &AppHandle,
    db: &Db,
    id: &str,
    options: &DownloadOptions,
    cancel: CancellationToken,
) -> AppResult<RunOutcome> {
    let yt_dlp = bin::resolve("yt-dlp");
    if !yt_dlp.is_present() {
        return Err(AppError::YtDlpNotFound);
    }

    let mut argv = build_args(options);
    // yt-dlp internal retries (these are *per-attempt*; the queue does outer retries).
    argv.insert(0, "--retries".into());
    argv.insert(1, "3".into());
    argv.insert(2, "--fragment-retries".into());
    argv.insert(3, "10".into());
    // Keep partial files so the next attempt can resume.
    argv.insert(4, "--continue".into());
    argv.insert(5, "--no-overwrites".into());

    // Point yt-dlp at our bundled ffmpeg if one is available.
    if let Some(ffmpeg_dir) = bin::ffmpeg_location() {
        argv.insert(6, "--ffmpeg-location".into());
        argv.insert(7, ffmpeg_dir.to_string_lossy().into_owned());
    }

    tracing::info!(id, bin = %yt_dlp.path.display(), ?argv, "spawning yt-dlp");

    let mut cmd = Command::new(&yt_dlp.path);
    cmd.args(&argv)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::YtDlpNotFound);
        }
        Err(err) => return Err(err.into()),
    };

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // stdout: parse progress, persist to DB, emit events
    {
        let app = app.clone();
        let db = db.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            loop {
                match reader.next_line().await {
                    Ok(Some(line)) => {
                        if let Some(update) = parse_line(&id, &line) {
                            let _ = db
                                .update_progress(
                                    &id,
                                    update.percent.unwrap_or(0.0),
                                    update.downloaded as i64,
                                    update.total.map(|t| t as i64),
                                    update.title.as_deref(),
                                )
                                .await;
                            let _ = app.emit(EVT_PROGRESS, update);
                        } else if let Some(rest) = line.strip_prefix(FILEPATH_PREFIX) {
                            // `__YTADLP_FILEPATH__\t<path>` — capture and persist.
                            let path = rest.trim_start_matches('\t').trim();
                            if !path.is_empty() {
                                let _ = db.set_output_path(&id, path).await;
                                tracing::info!(id = %id, %path, "captured output path");
                            }
                        } else if !line.is_empty() {
                            let _ = app.emit(
                                EVT_LOG,
                                DownloadLog { id: id.clone(), stream: "stdout", line },
                            );
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::warn!(error = ?e, "stdout read error");
                        break;
                    }
                }
            }
        });
    }

    // stderr: emit to UI, AND keep a rolling tail (last 12 non-empty lines)
    // so we can surface a meaningful error on non-zero exit.
    let stderr_tail: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::with_capacity(12)));
    {
        let app = app.clone();
        let id = id.to_string();
        let tail = stderr_tail.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if !line.trim().is_empty() {
                    // Mirror to tracing so dev terminal shows yt-dlp's complaint.
                    tracing::warn!(target: "yt-dlp", id = %id, "{}", line);
                    if let Ok(mut t) = tail.lock() {
                        t.push(line.clone());
                        let len = t.len();
                        if len > 12 {
                            t.drain(0..len - 12);
                        }
                    }
                }
                let _ = app.emit(
                    EVT_LOG,
                    DownloadLog { id: id.clone(), stream: "stderr", line },
                );
            }
        });
    }

    // Wait for exit OR cancellation.
    let outcome = tokio::select! {
        status = child.wait() => {
            match status {
                Ok(s) if s.success() => RunOutcome::Success,
                Ok(s) => {
                    let code = s.code().unwrap_or(-1);
                    let tail = stderr_tail
                        .lock()
                        .ok()
                        .map(|t| t.join("\n"))
                        .unwrap_or_default();

                    // Give the stdout reader a moment to flush the captured filepath.
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                    // Check if an output file was captured despite the error —
                    // this happens when the video downloads fine but a non-fatal
                    // post-processor (embed-thumbnail on ffmpeg-free) fails.
                    let has_output = db.get(id).await
                        .ok()
                        .flatten()
                        .and_then(|r| r.output_path)
                        .map(|p| std::path::Path::new(&p).exists())
                        .unwrap_or(false);

                    if has_output {
                        // Post-processing failed but the video exists — partial success.
                        let warning = tail
                            .lines()
                            .rev()
                            .find(|l| l.contains("ERROR") || l.contains("WARNING"))
                            .unwrap_or("Post-processing partially failed")
                            .trim()
                            .to_string();
                        tracing::warn!(
                            id, code,
                            "yt-dlp exited with code {} but output file exists — treating as partial success: {}",
                            code, warning
                        );
                        RunOutcome::PartialSuccess { warning }
                    } else {
                        RunOutcome::Failed {
                            code,
                            stderr_tail: tail,
                        }
                    }
                }
                Err(e) => return Err(AppError::Io(e)),
            }
        }
        _ = cancel.cancelled() => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            RunOutcome::Canceled
        }
    };

    Ok(outcome)
}
