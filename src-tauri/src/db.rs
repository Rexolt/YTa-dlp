use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, AppResult};
use crate::ytdlp::DownloadOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    pub id: String,
    pub url: String,
    pub title: Option<String>,
    pub status: String, // queued | downloading | paused | finished | error | canceled
    pub percent: f32,
    pub downloaded: i64,
    pub total: Option<i64>,
    pub error: Option<String>,
    pub output_path: Option<String>,
    pub attempts: i32,
    pub queue_position: i64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub options_json: String,
}

impl DownloadRecord {
    pub fn options(&self) -> AppResult<DownloadOptions> {
        serde_json::from_str(&self.options_json).map_err(AppError::from)
    }
}

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn open(path: PathBuf) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let opts = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .map_err(|e| AppError::Other(format!("db open: {e}")))?;

        sqlx::query(SCHEMA)
            .execute(&pool)
            .await
            .map_err(|e| AppError::Other(format!("db migrate: {e}")))?;

        Ok(Self { pool })
    }

    pub async fn insert_queued(
        &self,
        id: &str,
        url: &str,
        options: &DownloadOptions,
        queue_position: i64,
    ) -> AppResult<()> {
        let json = serde_json::to_string(options)?;
        sqlx::query(
            "INSERT INTO downloads (id, url, status, percent, downloaded, total, attempts,
             queue_position, created_at, options_json)
             VALUES (?, ?, 'queued', 0, 0, NULL, 0, ?, ?, ?)",
        )
        .bind(id)
        .bind(url)
        .bind(queue_position)
        .bind(Utc::now())
        .bind(json)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Other(format!("db insert: {e}")))?;
        Ok(())
    }

    pub async fn mark_started(&self, id: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE downloads SET status='downloading', started_at=?, attempts=attempts+1 WHERE id=?",
        )
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Other(format!("db mark_started: {e}")))?;
        Ok(())
    }

    pub async fn update_progress(
        &self,
        id: &str,
        percent: f32,
        downloaded: i64,
        total: Option<i64>,
        title: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE downloads SET percent=?, downloaded=?, total=COALESCE(?, total),
             title=COALESCE(?, title) WHERE id=?",
        )
        .bind(percent)
        .bind(downloaded)
        .bind(total)
        .bind(title)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Other(format!("db update_progress: {e}")))?;
        Ok(())
    }

    pub async fn set_output_path(&self, id: &str, path: &str) -> AppResult<()> {
        sqlx::query("UPDATE downloads SET output_path=? WHERE id=?")
            .bind(path)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Other(format!("db set_output_path: {e}")))?;
        Ok(())
    }

    pub async fn mark_status(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> AppResult<()> {
        let finished_at = matches!(status, "finished" | "error" | "canceled").then(Utc::now);
        sqlx::query(
            "UPDATE downloads SET status=?, error=?, finished_at=COALESCE(?, finished_at) WHERE id=?",
        )
        .bind(status)
        .bind(error)
        .bind(finished_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Other(format!("db mark_status: {e}")))?;
        Ok(())
    }

    pub async fn list(&self, limit: i64) -> AppResult<Vec<DownloadRecord>> {
        let rows = sqlx::query_as::<_, DownloadRecord>(
            "SELECT * FROM downloads ORDER BY
             CASE status WHEN 'downloading' THEN 0 WHEN 'queued' THEN 1
                         WHEN 'paused' THEN 2 ELSE 3 END,
             queue_position ASC, created_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Other(format!("db list: {e}")))?;
        Ok(rows)
    }

    pub async fn get(&self, id: &str) -> AppResult<Option<DownloadRecord>> {
        let row = sqlx::query_as::<_, DownloadRecord>("SELECT * FROM downloads WHERE id=?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Other(format!("db get: {e}")))?;
        Ok(row)
    }

    pub async fn next_queue_position(&self) -> AppResult<i64> {
        let row = sqlx::query("SELECT COALESCE(MAX(queue_position), 0) + 1 AS pos FROM downloads WHERE status IN ('queued','paused')")
            .fetch_one(&self.pool).await
            .map_err(|e| AppError::Other(format!("db next_pos: {e}")))?;
        Ok(row.get::<i64, _>("pos"))
    }

    pub async fn delete_finished(&self) -> AppResult<u64> {
        let res = sqlx::query("DELETE FROM downloads WHERE status IN ('finished','error','canceled')")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Other(format!("db clear: {e}")))?;
        Ok(res.rows_affected())
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM downloads WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Other(format!("db delete: {e}")))?;
        Ok(())
    }

    pub async fn reorder(&self, id: &str, new_position: i64) -> AppResult<()> {
        sqlx::query("UPDATE downloads SET queue_position=? WHERE id=?")
            .bind(new_position)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Other(format!("db reorder: {e}")))?;
        Ok(())
    }
}

// sqlx::FromRow manual impl to handle camelCase JSON column + chrono types
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for DownloadRecord {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(DownloadRecord {
            id: row.try_get("id")?,
            url: row.try_get("url")?,
            title: row.try_get("title")?,
            status: row.try_get("status")?,
            percent: row.try_get::<f32, _>("percent")?,
            downloaded: row.try_get::<i64, _>("downloaded")?,
            total: row.try_get("total")?,
            error: row.try_get("error")?,
            output_path: row.try_get("output_path")?,
            attempts: row.try_get::<i32, _>("attempts")?,
            queue_position: row.try_get::<i64, _>("queue_position")?,
            created_at: row.try_get("created_at")?,
            started_at: row.try_get("started_at")?,
            finished_at: row.try_get("finished_at")?,
            options_json: row.try_get("options_json")?,
        })
    }
}

pub fn default_db_path() -> PathBuf {
    if let Some(dirs) = directories::ProjectDirs::from("dev", "rexolt", "ytadlp") {
        return dirs.data_dir().join("downloads.db");
    }
    PathBuf::from("./downloads.db")
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS downloads (
    id              TEXT PRIMARY KEY,
    url             TEXT NOT NULL,
    title           TEXT,
    status          TEXT NOT NULL,
    percent         REAL NOT NULL DEFAULT 0,
    downloaded      INTEGER NOT NULL DEFAULT 0,
    total           INTEGER,
    error           TEXT,
    output_path     TEXT,
    attempts        INTEGER NOT NULL DEFAULT 0,
    queue_position  INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL,
    started_at      DATETIME,
    finished_at     DATETIME,
    options_json    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_downloads_status   ON downloads(status);
CREATE INDEX IF NOT EXISTS idx_downloads_queuepos ON downloads(queue_position);
"#;
