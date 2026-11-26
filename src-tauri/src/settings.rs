//! Persisted settings. One JSON file, written atomically, forward-compatible
//! through serde defaults.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::storage;

fn default_output_dir() -> String {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."))
        .to_string_lossy()
        .into_owned()
}

fn default_filename_template() -> String {
    "%(title).200s [%(id)s].%(ext)s".to_string()
}

fn default_quality() -> String {
    "best".to_string()
}

fn default_sublang() -> String {
    "en".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_retry_multiplier() -> f64 {
    2.0
}

fn default_compact() -> bool {
    false
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CookieBucket {
    pub id: String,
    pub name: String,
    pub file: String,
    pub domain: String,
    pub added_at: i64,
}

impl Default for CookieBucket {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            file: String::new(),
            domain: String::new(),
            added_at: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadRule {
    pub id: String,
    pub pattern: String,
    pub dir: String,
    /// Empty means "use the global setting".
    #[serde(default)]
    pub quality: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub preset_id: String,
}

impl Default for DownloadRule {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pattern: String::new(),
            dir: String::new(),
            quality: String::new(),
            mode: String::new(),
            priority: 0,
            preset_id: String::new(),
        }
    }
}

/// A reusable download configuration the user can apply from Home.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadPreset {
    pub id: String,
    pub name: String,
    pub quality: String,
    pub mode: String,
    pub dir: String,
    pub template: String,
    pub priority: i32,
    pub speed_limit_mbps: u32,
}

impl Default for DownloadPreset {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            quality: "best".to_string(),
            mode: "auto".to_string(),
            dir: String::new(),
            template: String::new(),
            priority: 0,
            speed_limit_mbps: 0,
        }
    }
}

/// An RSS feed whose new entries are queued automatically.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RssFeed {
    pub id: String,
    pub url: String,
    pub title: String,
    pub seen: Vec<String>,
    pub last_checked_at: i64,
}

impl Default for RssFeed {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            url: String::new(),
            title: String::new(),
            seen: Vec::new(),
            last_checked_at: 0,
        }
    }
}

/// A saved search that appears as a smart folder in the Library sidebar.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SmartFolder {
    pub id: String,
    pub name: String,
    pub query: String,
    pub kind: String,
}

impl Default for SmartFolder {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            query: String::new(),
            kind: "all".to_string(),
        }
    }
}

/// First matching rule wins. An empty field means no override.
pub fn resolve_profile<'a>(settings: &'a AppSettings, url: &str) -> Option<&'a DownloadRule> {
    let lower = url.to_ascii_lowercase();
    settings.download_rules.iter().find(|rule| {
        let pattern = rule.pattern.trim().to_ascii_lowercase();
        !pattern.is_empty() && lower.contains(&pattern)
    })
}

/// First matching rule wins; an explicitly chosen folder always beats a rule.
pub fn resolve_download_dir(settings: &AppSettings, url: &str, fallback: &str) -> String {
    let lower = url.to_ascii_lowercase();
    for rule in &settings.download_rules {
        let pattern = rule.pattern.trim().to_ascii_lowercase();
        if pattern.is_empty() || rule.dir.trim().is_empty() {
            continue;
        }
        if lower.contains(&pattern) {
            return rule.dir.clone();
        }
    }
    fallback.to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub output_dir: String,
    pub always_ask_path: bool,
    pub organize_by_platform: bool,
    pub skip_existing: bool,
    pub download_archive: bool,
    pub filename_template: String,
    pub video_quality: String,
    pub max_concurrent: u32,
    pub concurrent_fragments: u32,
    pub max_retries: u32,
    pub speed_limit_mbps: u32,
    pub embed_metadata: bool,
    pub embed_thumbnail: bool,
    pub write_subtitles: bool,
    pub subtitle_lang: String,
    pub sponsorblock: bool,
    pub split_by_chapters: bool,
    pub live_from_start: bool,
    pub theme: String,
    pub language: String,
    pub reduce_motion: bool,
    pub autostart: bool,
    pub onboarding_completed: bool,
    pub close_to_tray: bool,
    pub cookies_from_browser: String,
    pub cookie_file: String,
    pub proxy: String,
    pub notifications: bool,
    pub prevent_sleep: bool,
    pub ytdlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
    pub download_rules: Vec<DownloadRule>,
    pub cookie_buckets: Vec<CookieBucket>,
    pub cookie_trash: Vec<CookieBucket>,
    #[serde(default)]
    pub presets: Vec<DownloadPreset>,
    #[serde(default)]
    pub smart_folders: Vec<SmartFolder>,
    #[serde(default)]
    pub retry_delay_secs: u64,
    #[serde(default = "default_retry_multiplier")]
    pub retry_multiplier: f64,
    #[serde(default)]
    pub quiet_hours_enabled: bool,
    #[serde(default)]
    pub quiet_start_hour: u32,
    #[serde(default)]
    pub quiet_end_hour: u32,
    #[serde(default)]
    pub post_action: String,
    #[serde(default)]
    pub post_action_dir: String,
    #[serde(default)]
    pub webhook_url: String,
    #[serde(default)]
    pub webhook_events: String,
    #[serde(default)]
    pub api_enabled: bool,
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub watch_folder: String,
    #[serde(default)]
    pub clipboard_monitor: bool,
    #[serde(default)]
    pub resolve_shortlinks: bool,
    #[serde(default)]
    pub accent: String,
    #[serde(default = "default_compact")]
    pub compact_mode: bool,
    #[serde(default)]
    pub shortcuts: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub rss_feeds: Vec<RssFeed>,
    #[serde(default)]
    pub active_profile: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            always_ask_path: false,
            organize_by_platform: false,
            skip_existing: true,
            download_archive: false,
            filename_template: default_filename_template(),
            video_quality: default_quality(),
            max_concurrent: 2,
            concurrent_fragments: 8,
            max_retries: 3,
            speed_limit_mbps: 0,
            embed_metadata: true,
            embed_thumbnail: true,
            write_subtitles: false,
            subtitle_lang: default_sublang(),
            sponsorblock: false,
            split_by_chapters: false,
            live_from_start: false,
            theme: default_theme(),
            language: default_language(),
            reduce_motion: false,
            autostart: false,
            onboarding_completed: false,
            close_to_tray: false,
            cookies_from_browser: String::new(),
            cookie_file: String::new(),
            proxy: String::new(),
            notifications: true,
            prevent_sleep: true,
            ytdlp_path: None,
            ffmpeg_path: None,
            download_rules: Vec::new(),
            cookie_buckets: Vec::new(),
            cookie_trash: Vec::new(),
            presets: Vec::new(),
            smart_folders: Vec::new(),
            retry_delay_secs: 2,
            retry_multiplier: 2.0,
            quiet_hours_enabled: false,
            quiet_start_hour: 23,
            quiet_end_hour: 7,
            post_action: "none".to_string(),
            post_action_dir: String::new(),
            webhook_url: String::new(),
            webhook_events: "done,error".to_string(),
            api_enabled: false,
            api_token: String::new(),
            watch_folder: String::new(),
            clipboard_monitor: false,
            resolve_shortlinks: true,
            accent: "#00C896".to_string(),
            compact_mode: false,
            shortcuts: std::collections::HashMap::new(),
            rss_feeds: Vec::new(),
            active_profile: String::new(),
        }
    }
}

fn settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

pub fn load(data_dir: &Path) -> AppSettings {
    storage::read_json_or_default(&settings_path(data_dir))
}

pub fn save(data_dir: &Path, settings: &AppSettings) -> std::io::Result<()> {
    storage::write_json_atomic(&settings_path(data_dir), settings)
}

pub fn reset(data_dir: &Path) -> std::io::Result<AppSettings> {
    let fresh = AppSettings::default();
    save(data_dir, &fresh)?;
    Ok(fresh)
}
