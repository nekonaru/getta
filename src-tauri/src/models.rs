//! Data models shared across the backend. All of these are the frontend
//! contract too; field names are snake_case on both sides.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoQuality {
    pub label: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub index: u32,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub platform: String,
    pub supported: bool,
    pub title: Option<String>,
    pub author: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub media_kind: Option<String>,
    pub is_playlist: bool,
    pub entry_count: Option<u32>,
    #[serde(default)]
    pub qualities: Vec<VideoQuality>,
    #[serde(default)]
    pub playlist_entries: Vec<PlaylistEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadMode {
    Auto,
    Audio,
    Mute,
    Subs,
}

impl Default for DownloadMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl DownloadMode {
    pub fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "audio" => Self::Audio,
            "mute" => Self::Mute,
            "subs" => Self::Subs,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Audio => "audio",
            Self::Mute => "mute",
            Self::Subs => "subs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueStatus {
    Queued,
    Active,
    Paused,
    Done,
    Error,
}

impl QueueStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Done => "done",
            Self::Error => "error",
        }
    }
}

/// Serialized view of a queue item for the frontend.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueItemInfo {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub platform: String,
    pub status: String,
    pub error_message: Option<String>,
    pub retryable: bool,
    pub percent: f64,
    pub speed_bps: u64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub phase: Option<String>,
    pub file_path: Option<String>,
    pub file_count: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub author: Option<String>,
    pub duration_seconds: Option<f64>,
    pub quality: Option<String>,
    pub download_mode: Option<String>,
    pub format_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChapterMarker {
    pub title: Option<String>,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FollowChannel {
    pub id: String,
    pub url: String,
    pub title: String,
    pub added_at: i64,
    pub last_checked_at: Option<i64>,
    pub auto_download: bool,
    #[serde(default)]
    pub seen_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChannelEntry {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FollowCheckResult {
    pub enqueued: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryEntry {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub download_mode: String,
    pub quality: String,
    pub format_id: Option<String>,
    pub playlist_items: Option<String>,
    pub output_dir: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub url: String,
    pub platform: String,
    pub title: String,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub success: bool,
    pub error: Option<String>,
    pub completed_at: i64,
    pub thumbnail_url: Option<String>,
    pub quality: Option<String>,
    pub download_mode: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub found: bool,
    pub version: Option<String>,
    pub source: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngineStatus {
    pub ytdlp: DependencyInfo,
    pub ffmpeg: DependencyInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartOptions {
    pub url: String,
    #[serde(default)]
    pub download_mode: String,
    #[serde(default)]
    pub quality: String,
    pub format_id: Option<String>,
    pub playlist_items: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    pub input: String,
    pub output: Option<String>,
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolFinishedEvent {
    pub tool: String,
    pub success: bool,
    pub output: Option<String>,
    pub message: Option<String>,
}
