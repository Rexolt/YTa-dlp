use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("yt-dlp not found on PATH. Please install yt-dlp.")]
    YtDlpNotFound,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Download {0} not found")]
    UnknownDownload(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("yt-dlp exited with code {0}")]
    NonZeroExit(i32),

    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
