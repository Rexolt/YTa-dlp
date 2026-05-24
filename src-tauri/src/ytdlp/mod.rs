pub mod args;
pub mod progress;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoContainer {
    Mp4,
    Mkv,
    Webm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioContainer {
    Mp3,
    Flac,
    Wav,
    M4a,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoQuality {
    /// e.g. 4320 (8K), 2160 (4K), 1440, 1080, 720
    pub max_height: u32,
    /// e.g. 30, 60
    pub fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioQuality {
    /// 128, 192, 320
    pub bitrate_kbps: u32,
    pub normalize: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleMode {
    /// Save sidecar file (.srt/.vtt) next to the video, no embedding.
    Sidecar,
    /// Embed as a selectable soft subtitle track inside the container.
    Embed,
    /// Burn into the video stream (visual overlay; ffmpeg subtitles filter).
    BurnIn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleOptions {
    pub languages: Vec<String>,
    pub auto_generated: bool,
    pub mode: SubtitleMode,
    /// "srt" | "vtt" — used for sidecar writes
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataOptions {
    pub embed_thumbnail: bool,
    pub keep_chapters: bool,
    pub embed_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SponsorBlockMode {
    Off,
    Mark,   // chapters only
    Remove, // cut from output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SponsorBlockOptions {
    pub mode: SponsorBlockMode,
    /// sponsor, intro, outro, selfpromo, interaction, music_offtopic, preview, filler
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkOptions {
    pub proxy: Option<String>,
    /// e.g. "2M" for 2 MiB/s
    pub rate_limit: Option<String>,
    /// "chrome" | "firefox" | "edge" | "brave" | "safari"
    pub cookies_from_browser: Option<String>,
    pub concurrent_fragments: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistOptions {
    pub enabled: bool,
    /// e.g. "1-5,7,10-12"
    pub items: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOptions {
    pub url: String,
    pub output_dir: String,
    pub kind: MediaKind,
    pub video_container: Option<VideoContainer>,
    pub audio_container: Option<AudioContainer>,
    pub video: Option<VideoQuality>,
    pub audio: Option<AudioQuality>,
    pub subtitles: Option<SubtitleOptions>,
    pub metadata: MetadataOptions,
    pub sponsorblock: SponsorBlockOptions,
    pub network: NetworkOptions,
    pub playlist: PlaylistOptions,
}
