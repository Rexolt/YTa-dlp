use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// SponsorBlock public API endpoint (community-run; no auth needed).
const SB_BASE: &str = "https://sponsor.ajay.app/api/skipSegments";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SponsorSegment {
    pub uuid: String,
    pub category: String,
    /// "skip" | "mute" | "chapter" | "poi" | "full"
    pub action_type: String,
    pub start: f64,
    pub end: f64,
    pub video_duration: Option<f64>,
    pub votes: Option<i32>,
    pub locked: Option<i32>,
    pub description: Option<String>,
}

/// Raw response shape from the SponsorBlock API.
#[derive(Debug, Deserialize)]
struct RawSegment {
    #[serde(rename = "UUID")]
    uuid: String,
    category: String,
    #[serde(rename = "actionType")]
    action_type: String,
    segment: [f64; 2],
    #[serde(rename = "videoDuration")]
    video_duration: Option<f64>,
    votes: Option<i32>,
    locked: Option<i32>,
    description: Option<String>,
}

/// Best-effort YouTube video id extractor. Supports:
///   - https://www.youtube.com/watch?v=ID
///   - https://youtu.be/ID
///   - https://www.youtube.com/shorts/ID
///   - https://www.youtube.com/embed/ID
///   - https://music.youtube.com/watch?v=ID
pub fn extract_video_id(url: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)
            (?:
                (?:v|vi)=([A-Za-z0-9_-]{11})       # ?v=ID
              | youtu\.be/([A-Za-z0-9_-]{11})       # youtu.be/ID
              | /shorts/([A-Za-z0-9_-]{11})         # /shorts/ID
              | /embed/([A-Za-z0-9_-]{11})          # /embed/ID
            )
            ",
        )
        .unwrap()
    });

    RE.captures(url).and_then(|c| {
        (1..=4)
            .filter_map(|i| c.get(i))
            .map(|m| m.as_str().to_string())
            .next()
    })
}

/// All categories yt-dlp knows about; mirrors `--sponsorblock-mark/remove all`.
pub const ALL_CATEGORIES: &[&str] = &[
    "sponsor",
    "intro",
    "outro",
    "selfpromo",
    "interaction",
    "music_offtopic",
    "preview",
    "filler",
];

/// Fetch SponsorBlock segments for a given video URL.
/// Returns Ok(vec![]) when there are no segments (HTTP 404 from the API).
pub async fn fetch_segments(url: &str, categories: &[String]) -> AppResult<Vec<SponsorSegment>> {
    let Some(video_id) = extract_video_id(url) else {
        return Ok(vec![]);
    };

    // Build categories array param (JSON-encoded list per API contract).
    let cats: Vec<&str> = if categories.is_empty() {
        ALL_CATEGORIES.iter().copied().collect()
    } else {
        categories.iter().map(|s| s.as_str()).collect()
    };
    let cats_json = serde_json::to_string(&cats).unwrap_or_else(|_| "[]".to_string());

    let endpoint = format!(
        "{base}?videoID={id}&categories={cats}",
        base = SB_BASE,
        id = urlencoding::encode(&video_id),
        cats = urlencoding::encode(&cats_json),
    );

    let client = reqwest::Client::builder()
        .user_agent(concat!("YTa-dlp/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Other(format!("sb client: {e}")))?;

    let resp = client
        .get(&endpoint)
        .send()
        .await
        .map_err(|e| AppError::Other(format!("sb request: {e}")))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        // SponsorBlock returns 404 when no segments exist for this video.
        return Ok(vec![]);
    }
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "SponsorBlock returned HTTP {}",
            resp.status()
        )));
    }

    let raw: Vec<RawSegment> = resp
        .json()
        .await
        .map_err(|e| AppError::Other(format!("sb json: {e}")))?;

    Ok(raw
        .into_iter()
        .map(|r| SponsorSegment {
            uuid: r.uuid,
            category: r.category,
            action_type: r.action_type,
            start: r.segment[0],
            end: r.segment[1],
            video_duration: r.video_duration,
            votes: r.votes,
            locked: r.locked,
            description: r.description,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_video_ids() {
        let cases = [
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://youtu.be/dQw4w9WgXcQ?si=foo", "dQw4w9WgXcQ"),
            ("https://www.youtube.com/shorts/dQw4w9WgXcQ", "dQw4w9WgXcQ"),
            ("https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=RD", "dQw4w9WgXcQ"),
        ];
        for (url, want) in cases {
            assert_eq!(extract_video_id(url).as_deref(), Some(want), "url={url}");
        }
        assert!(extract_video_id("https://example.com/not-youtube").is_none());
    }
}
