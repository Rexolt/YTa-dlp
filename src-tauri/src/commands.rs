use std::process::Stdio;

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio::process::Command;

use crate::bin::{self, BinarySource};
use crate::db::DownloadRecord;
use crate::error::{AppError, AppResult};
use crate::queue::QueueManager;
use crate::sponsorblock::{self, SponsorSegment};
use crate::ytdlp::DownloadOptions;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub title: Option<String>,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    pub lang: String,
    pub name: Option<String>,
    /// "manual" | "auto"
    pub kind: &'static str,
    pub formats: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatInfo {
    pub format_id: String,
    pub ext: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub fps: Option<f64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    /// Total bitrate kbps (yt-dlp `tbr` * 1).
    pub tbr_kbps: Option<f64>,
    /// Audio bitrate (kbps).
    pub abr_kbps: Option<f64>,
    pub filesize: Option<u64>,
    pub format_note: Option<String>,
    /// "video" | "audio" | "video+audio"
    pub kind: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub uploader_url: Option<String>,
    pub channel: Option<String>,
    pub description: Option<String>,
    pub duration_secs: Option<f64>,
    pub thumbnail: Option<String>,
    /// All thumbnails sorted by size (smallest → largest), for picking the best.
    pub thumbnails: Vec<String>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub upload_date: Option<String>,
    pub age_limit: Option<u32>,
    pub live_status: Option<String>,
    pub webpage_url: Option<String>,
    pub is_playlist: bool,
    pub playlist_count: Option<u64>,
    pub chapters: Vec<ChapterInfo>,
    pub subtitles: Vec<SubtitleTrack>,
    pub sponsor_segments: Vec<SponsorSegment>,
    pub formats: Vec<FormatInfo>,
    /// Distinct video heights actually available, descending.
    pub available_heights: Vec<u32>,
    /// Distinct fps values actually available (typically 30 / 60).
    pub available_fps: Vec<u32>,
    /// Distinct audio bitrates available (kbps).
    pub available_audio_bitrates: Vec<u32>,
    pub has_video: bool,
    pub has_audio: bool,
}

fn parse_chapters(raw: &serde_json::Value) -> Vec<ChapterInfo> {
    raw.get("chapters")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let start = c.get("start_time")?.as_f64()?;
                    let end = c.get("end_time")?.as_f64()?;
                    Some(ChapterInfo {
                        title: c.get("title").and_then(|t| t.as_str()).map(String::from),
                        start,
                        end,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn classify_format(vcodec: Option<&str>, acodec: Option<&str>) -> &'static str {
    let has_v = vcodec.map(|c| c != "none" && !c.is_empty()).unwrap_or(false);
    let has_a = acodec.map(|c| c != "none" && !c.is_empty()).unwrap_or(false);
    match (has_v, has_a) {
        (true, true) => "video+audio",
        (true, false) => "video",
        (false, true) => "audio",
        _ => "video", // unknown — treat as video (storyboards filtered separately)
    }
}

fn parse_formats(raw: &serde_json::Value) -> Vec<FormatInfo> {
    let Some(arr) = raw.get("formats").and_then(|v| v.as_array()) else {
        return vec![];
    };
    arr.iter()
        .filter_map(|f| {
            let format_id = f.get("format_id").and_then(|v| v.as_str())?.to_string();
            let ext = f.get("ext").and_then(|v| v.as_str()).map(String::from);
            let vcodec = f.get("vcodec").and_then(|v| v.as_str());
            let acodec = f.get("acodec").and_then(|v| v.as_str());

            // Filter out yt-dlp's storyboard / metadata pseudo-formats.
            if ext.as_deref() == Some("mhtml") {
                return None;
            }
            // protocol m3u8_native etc. is fine; we only skip obvious junk.
            if let Some(proto) = f.get("protocol").and_then(|v| v.as_str()) {
                if proto.contains("mhtml") {
                    return None;
                }
            }

            let kind = classify_format(vcodec, acodec);
            Some(FormatInfo {
                format_id,
                ext,
                height: f.get("height").and_then(|v| v.as_u64()).map(|h| h as u32),
                width: f.get("width").and_then(|v| v.as_u64()).map(|w| w as u32),
                fps: f.get("fps").and_then(|v| v.as_f64()),
                vcodec: vcodec.map(String::from),
                acodec: acodec.map(String::from),
                tbr_kbps: f.get("tbr").and_then(|v| v.as_f64()),
                abr_kbps: f.get("abr").and_then(|v| v.as_f64()),
                filesize: f
                    .get("filesize")
                    .and_then(|v| v.as_u64())
                    .or_else(|| f.get("filesize_approx").and_then(|v| v.as_u64())),
                format_note: f.get("format_note").and_then(|v| v.as_str()).map(String::from),
                kind,
            })
        })
        .collect()
}

fn parse_thumbnails(raw: &serde_json::Value) -> Vec<String> {
    let Some(arr) = raw.get("thumbnails").and_then(|v| v.as_array()) else {
        return vec![];
    };
    let mut tagged: Vec<(u64, String)> = arr
        .iter()
        .filter_map(|t| {
            let url = t.get("url").and_then(|v| v.as_str())?.to_string();
            let w = t.get("width").and_then(|v| v.as_u64()).unwrap_or(0);
            let h = t.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
            Some((w.saturating_mul(h.max(1)), url))
        })
        .collect();
    tagged.sort_by_key(|(area, _)| *area);
    tagged.into_iter().map(|(_, url)| url).collect()
}

fn parse_subtitle_map(
    obj: Option<&serde_json::Value>,
    kind: &'static str,
) -> Vec<SubtitleTrack> {
    let Some(map) = obj.and_then(|v| v.as_object()) else { return vec![]; };
    map.iter()
        .map(|(lang, tracks)| {
            let formats: Vec<String> = tracks
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.get("ext").and_then(|e| e.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let name = tracks
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|t| t.get("name").and_then(|n| n.as_str()).map(String::from));
            SubtitleTrack { lang: lang.clone(), name, kind, formats }
        })
        .collect()
}

#[tauri::command]
pub async fn probe_url(url: String) -> AppResult<ProbeResult> {
    let yt_dlp = bin::resolve("yt-dlp");
    if !yt_dlp.is_present() {
        return Err(AppError::YtDlpNotFound);
    }
    let output = Command::new(&yt_dlp.path)
        .args(["-J", "--no-warnings", "--no-playlist", "--skip-download", &url])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::YtDlpNotFound,
            _ => AppError::Io(e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AppError::Other(format!(
            "probe failed: {}",
            stderr.lines().last().unwrap_or("unknown error")
        )));
    }

    let raw: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let chapters = parse_chapters(&raw);
    let mut subtitles = parse_subtitle_map(raw.get("subtitles"), "manual");
    subtitles.extend(parse_subtitle_map(raw.get("automatic_captions"), "auto"));
    subtitles.sort_by(|a, b| a.kind.cmp(b.kind).then_with(|| a.lang.cmp(&b.lang)));

    let formats = parse_formats(&raw);
    let thumbnails = parse_thumbnails(&raw);

    // Derive available_heights — only from formats that actually have video.
    let mut heights: Vec<u32> = formats
        .iter()
        .filter(|f| f.kind != "audio")
        .filter_map(|f| f.height)
        .collect();
    heights.sort_unstable();
    heights.dedup();
    heights.reverse();

    // Available fps (rounded to int; 30 / 60 are typical).
    let mut fps: Vec<u32> = formats
        .iter()
        .filter(|f| f.kind != "audio")
        .filter_map(|f| f.fps.map(|v| v.round() as u32))
        .filter(|&v| v > 0)
        .collect();
    fps.sort_unstable();
    fps.dedup();

    // Available audio bitrates (rounded to nearest 32 kbps bucket).
    let mut abr: Vec<u32> = formats
        .iter()
        .filter(|f| f.kind != "video")
        .filter_map(|f| f.abr_kbps.or(f.tbr_kbps).map(|v| v.round() as u32))
        .filter(|&v| v > 0)
        .collect();
    abr.sort_unstable();
    abr.dedup();

    let has_video = formats.iter().any(|f| f.kind != "audio" && f.height.is_some());
    let has_audio = formats
        .iter()
        .any(|f| f.kind != "video" && (f.abr_kbps.is_some() || f.acodec.is_some()));

    // Fetch SponsorBlock segments concurrently — failure is non-fatal.
    let sponsor_segments = sponsorblock::fetch_segments(&url, &[])
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = ?e, "sponsorblock fetch failed (non-fatal)");
            vec![]
        });

    // Pick the best thumbnail: prefer the largest from the list, fall back to
    // the top-level `thumbnail` field.
    let thumbnail = thumbnails
        .last()
        .cloned()
        .or_else(|| raw.get("thumbnail").and_then(|v| v.as_str()).map(String::from));

    Ok(ProbeResult {
        title: raw.get("title").and_then(|v| v.as_str()).map(String::from),
        uploader: raw.get("uploader").and_then(|v| v.as_str()).map(String::from),
        uploader_url: raw.get("uploader_url").and_then(|v| v.as_str()).map(String::from),
        channel: raw.get("channel").and_then(|v| v.as_str()).map(String::from),
        description: raw.get("description").and_then(|v| v.as_str()).map(String::from),
        duration_secs: raw.get("duration").and_then(|v| v.as_f64()),
        thumbnail,
        thumbnails,
        view_count: raw.get("view_count").and_then(|v| v.as_u64()),
        like_count: raw.get("like_count").and_then(|v| v.as_u64()),
        upload_date: raw.get("upload_date").and_then(|v| v.as_str()).map(String::from),
        age_limit: raw.get("age_limit").and_then(|v| v.as_u64()).map(|n| n as u32),
        live_status: raw.get("live_status").and_then(|v| v.as_str()).map(String::from),
        webpage_url: raw.get("webpage_url").and_then(|v| v.as_str()).map(String::from),
        is_playlist: raw.get("_type").and_then(|v| v.as_str()) == Some("playlist"),
        playlist_count: raw.get("playlist_count").and_then(|v| v.as_u64()),
        chapters,
        subtitles,
        sponsor_segments,
        formats,
        available_heights: heights,
        available_fps: fps,
        available_audio_bitrates: abr,
        has_video,
        has_audio,
    })
}

#[tauri::command]
pub async fn start_download(
    queue: State<'_, QueueManager>,
    options: DownloadOptions,
) -> AppResult<String> {
    queue.enqueue(options).await
}

#[tauri::command]
pub async fn cancel_download(queue: State<'_, QueueManager>, id: String) -> AppResult<bool> {
    queue.cancel(&id).await
}

#[tauri::command]
pub async fn pause_download(queue: State<'_, QueueManager>, id: String) -> AppResult<bool> {
    queue.pause(&id).await
}

#[tauri::command]
pub async fn resume_download(queue: State<'_, QueueManager>, id: String) -> AppResult<bool> {
    queue.resume(&id).await
}

#[tauri::command]
pub async fn retry_download(queue: State<'_, QueueManager>, id: String) -> AppResult<bool> {
    queue.retry(&id).await
}

#[tauri::command]
pub async fn list_downloads(
    queue: State<'_, QueueManager>,
    limit: Option<i64>,
) -> AppResult<Vec<DownloadRecord>> {
    queue.db().list(limit.unwrap_or(500)).await
}

#[tauri::command]
pub async fn delete_download(queue: State<'_, QueueManager>, id: String) -> AppResult<()> {
    // Best-effort cancel first (no-op if not running).
    let _ = queue.cancel(&id).await;
    queue.db().delete(&id).await
}

#[tauri::command]
pub async fn clear_history(queue: State<'_, QueueManager>) -> AppResult<u64> {
    queue.db().delete_finished().await
}

#[tauri::command]
pub async fn set_max_concurrent(
    queue: State<'_, QueueManager>,
    n: usize,
) -> AppResult<()> {
    queue.set_max_concurrent(n).await
}

#[tauri::command]
pub async fn reorder_download(
    queue: State<'_, QueueManager>,
    id: String,
    position: i64,
) -> AppResult<()> {
    queue.reorder(&id, position).await
}

#[tauri::command]
pub async fn ytdlp_version() -> AppResult<String> {
    let yt_dlp = bin::resolve("yt-dlp");
    if !yt_dlp.is_present() {
        return Err(AppError::YtDlpNotFound);
    }
    let output = Command::new(&yt_dlp.path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::YtDlpNotFound,
            _ => AppError::Io(e),
        })?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub name: &'static str,
    pub available: bool,
    pub source: String, // "packaged" | "devSidecar" | "systemPath" | "missing"
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub yt_dlp: ToolStatus,
    pub ffmpeg: ToolStatus,
    pub ffprobe: ToolStatus,
}

async fn probe_tool_version(
    resolved: &bin::ResolvedBinary,
    arg: &str,
) -> Option<String> {
    if !resolved.is_present() {
        return None;
    }
    let out = Command::new(&resolved.path).arg(arg).output().await.ok()?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let first = s.lines().next().unwrap_or("").trim().to_string();
    if first.is_empty() { None } else { Some(first) }
}

fn source_to_str(s: BinarySource) -> &'static str {
    match s {
        BinarySource::Packaged => "packaged",
        BinarySource::DevSidecar => "devSidecar",
        BinarySource::SystemPath => "systemPath",
        BinarySource::Missing => "missing",
    }
}

async fn status_for(name: &'static str, version_arg: &str) -> ToolStatus {
    let r = bin::resolve(name);
    let version = probe_tool_version(&r, version_arg).await;
    ToolStatus {
        name,
        available: r.is_present(),
        source: source_to_str(r.source).into(),
        path: r.is_present().then(|| r.path.display().to_string()),
        version,
    }
}

#[tauri::command]
pub async fn check_environment() -> AppResult<EnvironmentReport> {
    let (yt_dlp, ffmpeg, ffprobe) = tokio::join!(
        status_for("yt-dlp", "--version"),
        status_for("ffmpeg", "-version"),
        status_for("ffprobe", "-version"),
    );
    Ok(EnvironmentReport { yt_dlp, ffmpeg, ffprobe })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub success: bool,
    pub output: String,
}

/// Open a native directory picker. Returns the absolute path, or null if the
/// user cancelled.
#[tauri::command]
pub async fn pick_output_directory(app: AppHandle) -> AppResult<Option<String>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose output folder")
        .pick_folder(move |path| {
            let _ = tx.send(path);
        });
    let result = rx
        .await
        .map_err(|e| AppError::Other(format!("dialog channel: {e}")))?;
    Ok(result.and_then(|p| p.into_path().ok().map(|pb| pb.to_string_lossy().into_owned())))
}

/// Return the user's default download directory, e.g. `~/Downloads`.
#[tauri::command]
pub fn default_download_dir() -> AppResult<String> {
    let dir = directories::UserDirs::new()
        .and_then(|d| d.download_dir().map(|p| p.join("YTa-dlp")))
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = std::path::PathBuf::from(h);
                p.push("Downloads");
                p.push("YTa-dlp");
                p
            })
        })
        .ok_or_else(|| AppError::Other("could not resolve download dir".into()))?;
    Ok(dir.to_string_lossy().into_owned())
}

/// Reveal a file (or its containing folder if a file path) in the OS file manager.
#[tauri::command]
pub async fn reveal_in_file_manager(path: String) -> AppResult<()> {
    let target = std::path::PathBuf::from(&path);
    let folder = if target.is_file() {
        target.parent().map(|p| p.to_path_buf()).unwrap_or(target)
    } else {
        target
    };
    let folder_str = folder.to_string_lossy().to_string();

    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";

    Command::new(program)
        .arg(&folder_str)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(AppError::Io)?;
    Ok(())
}

#[tauri::command]
pub async fn update_ytdlp() -> AppResult<UpdateResult> {
    let yt_dlp = bin::resolve("yt-dlp");
    if !yt_dlp.is_present() {
        return Err(AppError::YtDlpNotFound);
    }
    let out = Command::new(&yt_dlp.path)
        .args(["-U", "--no-colors"])
        .output()
        .await
        .map_err(AppError::Io)?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(UpdateResult { success: out.status.success(), output: combined })
}
