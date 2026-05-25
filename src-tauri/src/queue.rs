use std::sync::Arc;
use std::time::Duration;

use dashmap::{DashMap, DashSet};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Notify, RwLock, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::Db;
use crate::downloader::{run_ytdlp_once, RunOutcome, EVT_FINISHED, EVT_STARTED};
use crate::error::{AppError, AppResult};
use crate::ytdlp::DownloadOptions;

const DEFAULT_MAX_CONCURRENT: usize = 2;
const MAX_ATTEMPTS: i32 = 3;
const BASE_BACKOFF_MS: u64 = 1500;

/// Events emitted by the queue layer (separate from per-download events).
pub const EVT_QUEUE_CHANGED: &str = "queue://changed";
/// Fired immediately after a download is enqueued so the UI can render a
/// "queued" card without waiting for the dispatcher to pick it up.
pub const EVT_QUEUED: &str = "queue://queued";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueueChanged {
    pub max_concurrent: usize,
    pub active: usize,
    pub queued: usize,
}

#[derive(Clone)]
pub struct QueueManager {
    inner: Arc<Inner>,
}

struct Inner {
    db: Db,
    app: AppHandle,
    semaphore: Arc<Semaphore>,
    max_concurrent: RwLock<usize>,
    active: DashMap<String, CancellationToken>,
    paused: DashSet<String>,
    notify: Notify,
}

impl QueueManager {
    pub fn new(db: Db, app: AppHandle) -> Self {
        let mgr = Self {
            inner: Arc::new(Inner {
                db,
                app,
                semaphore: Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT)),
                max_concurrent: RwLock::new(DEFAULT_MAX_CONCURRENT),
                active: DashMap::new(),
                paused: DashSet::new(),
                notify: Notify::new(),
            }),
        };
        mgr.spawn_dispatcher();
        mgr
    }

    /// Enqueue a download. Returns the new id.
    pub async fn enqueue(&self, options: DownloadOptions) -> AppResult<String> {
        let id = Uuid::new_v4().to_string();
        let pos = self.inner.db.next_queue_position().await?;
        self.inner
            .db
            .insert_queued(&id, &options.url, &options, pos)
            .await?;
        // Emit a snapshot of the freshly created record so the UI can show
        // the queued card immediately, without waiting for the dispatcher.
        if let Ok(Some(rec)) = self.inner.db.get(&id).await {
            let _ = self.inner.app.emit(EVT_QUEUED, rec);
        }
        self.notify_changed().await;
        self.inner.notify.notify_one();
        Ok(id)
    }

    pub async fn cancel(&self, id: &str) -> AppResult<bool> {
        // Already running? cancel the child.
        if let Some((_, token)) = self.inner.active.remove(id) {
            token.cancel();
            // status will be set to 'canceled' by the worker on shutdown
            return Ok(true);
        }
        // Otherwise: queued or paused — just delete from DB / mark canceled.
        if let Some(rec) = self.inner.db.get(id).await? {
            if matches!(rec.status.as_str(), "queued" | "paused") {
                self.inner.paused.remove(id);
                self.inner.db.mark_status(id, "canceled", None).await?;
                self.notify_changed().await;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn pause(&self, id: &str) -> AppResult<bool> {
        if let Some((_, token)) = self.inner.active.remove(id) {
            self.inner.paused.insert(id.to_string());
            self.inner.db.mark_status(id, "paused", None).await?;
            token.cancel();
            self.notify_changed().await;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn resume(&self, id: &str) -> AppResult<bool> {
        self.inner.paused.remove(id);
        if let Some(rec) = self.inner.db.get(id).await? {
            if rec.status == "paused" || rec.status == "error" || rec.status == "canceled" {
                self.inner.db.mark_status(id, "queued", None).await?;
                self.inner.notify.notify_one();
                self.notify_changed().await;
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn retry(&self, id: &str) -> AppResult<bool> {
        self.resume(id).await
    }

    pub async fn set_max_concurrent(&self, n: usize) -> AppResult<()> {
        let n = n.clamp(1, 16);
        // IMPORTANT: drop the write guard before calling notify_changed,
        // which acquires a read lock on the same RwLock. Holding the write
        // guard across that await would deadlock the whole backend.
        {
            let mut cur = self.inner.max_concurrent.write().await;
            if n > *cur {
                self.inner.semaphore.add_permits(n - *cur);
            } else if n < *cur {
                // Reduce by forgetting permits as they come back.
                let diff = *cur - n;
                let sem = self.inner.semaphore.clone();
                tokio::spawn(async move {
                    for _ in 0..diff {
                        if let Ok(p) = sem.clone().acquire_owned().await {
                            p.forget();
                        }
                    }
                });
            }
            *cur = n;
        } // guard dropped here
        self.notify_changed().await;
        Ok(())
    }

    pub async fn reorder(&self, id: &str, position: i64) -> AppResult<()> {
        self.inner.db.reorder(id, position).await?;
        self.notify_changed().await;
        Ok(())
    }

    pub fn db(&self) -> &Db {
        &self.inner.db
    }

    async fn notify_changed(&self) {
        let max = *self.inner.max_concurrent.read().await;
        let active = self.inner.active.len();
        let queued = self
            .inner
            .db
            .list(1000)
            .await
            .map(|v| v.iter().filter(|r| r.status == "queued").count())
            .unwrap_or(0);
        let _ = self.inner.app.emit(
            EVT_QUEUE_CHANGED,
            QueueChanged { max_concurrent: max, active, queued },
        );
    }

    fn spawn_dispatcher(&self) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            loop {
                // Acquire a permit (waits if at concurrency cap).
                let permit = match inner.semaphore.clone().acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => break, // semaphore closed
                };

                // Find the next queued record.
                let next = match inner.db.list(1).await {
                    Ok(list) => list.into_iter().find(|r| r.status == "queued"),
                    Err(e) => {
                        tracing::warn!(?e, "db list failed");
                        None
                    }
                };

                let rec = match next {
                    Some(r) => r,
                    None => {
                        drop(permit);
                        inner.notify.notified().await;
                        continue;
                    }
                };

                let mgr = QueueManager { inner: inner.clone() };
                tokio::spawn(async move {
                    let _permit = permit;
                    mgr.run_with_retries(rec).await;
                });
            }
        });
    }

    async fn run_with_retries(&self, rec: crate::db::DownloadRecord) {
        let options = match rec.options() {
            Ok(o) => o,
            Err(e) => {
                let _ = self
                    .inner
                    .db
                    .mark_status(&rec.id, "error", Some(&format!("bad options: {e}")))
                    .await;
                self.notify_changed().await;
                return;
            }
        };

        let token = CancellationToken::new();
        self.inner.active.insert(rec.id.clone(), token.clone());
        let _ = self.inner.db.mark_started(&rec.id).await;
        let _ = self.inner.app.emit(
            EVT_STARTED,
            crate::downloader::DownloadStarted { id: rec.id.clone(), url: rec.url.clone() },
        );
        self.notify_changed().await;

        let mut attempt = rec.attempts.max(0) as u32;
        let outcome = loop {
            attempt = attempt.saturating_add(1);
            let res = run_ytdlp_once(
                &self.inner.app,
                &self.inner.db,
                &rec.id,
                &options,
                token.clone(),
            )
            .await;

            match res {
                Ok(RunOutcome::Success) => break Ok(()),
                Ok(RunOutcome::PartialSuccess { warning }) => {
                    tracing::warn!(id=%rec.id, %warning, "download partial success — will not retry");
                    break Ok(());
                }
                Ok(RunOutcome::Canceled) => break Err(AppError::Other("canceled".into())),
                Ok(RunOutcome::Failed { code, stderr_tail: _ })
                | Err(AppError::NonZeroExit(code))
                    if attempt < MAX_ATTEMPTS as u32 && !token.is_cancelled() =>
                {
                    let delay = BASE_BACKOFF_MS * 2u64.pow(attempt - 1);
                    tracing::warn!(id=%rec.id, code, attempt, "retrying after {}ms", delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    continue;
                }
                Ok(RunOutcome::Failed { code, stderr_tail }) => {
                    // Distill the final stderr line containing "ERROR:" if any,
                    // otherwise the very last non-empty line.
                    let summary = stderr_tail
                        .lines()
                        .rev()
                        .find(|l| l.contains("ERROR"))
                        .or_else(|| stderr_tail.lines().rev().find(|l| !l.trim().is_empty()))
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let msg = if summary.is_empty() {
                        format!("yt-dlp exited with code {code}")
                    } else {
                        format!("yt-dlp exited with code {code}: {summary}")
                    };
                    tracing::error!(id=%rec.id, code, "{msg}");
                    break Err(AppError::Other(msg));
                }
                Err(e) => break Err(e),
            }
        };

        // Final status reconciliation.
        self.inner.active.remove(&rec.id);
        let canceled = token.is_cancelled();
        let is_paused = self.inner.paused.contains(&rec.id);

        let (status, error): (&str, Option<String>) = match (&outcome, canceled, is_paused) {
            (_, _, true) => ("paused", None),
            (Ok(_), _, _) => ("finished", None),
            (Err(_), true, _) => ("canceled", None),
            (Err(e), _, _) => ("error", Some(e.to_string())),
        };

        let _ = self.inner.db.mark_status(&rec.id, status, error.as_deref()).await;
        // Look up the captured output path (set during the run by downloader.rs).
        let output_path = self
            .inner
            .db
            .get(&rec.id)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.output_path);
        let _ = self.inner.app.emit(
            EVT_FINISHED,
            crate::downloader::DownloadFinished {
                id: rec.id.clone(),
                success: status == "finished",
                exit_code: None,
                error,
                canceled: status == "canceled",
                output_path,
            },
        );
        self.notify_changed().await;
        self.inner.notify.notify_one();
    }
}
