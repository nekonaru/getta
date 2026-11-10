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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadRule {
    pub id: String,
    pub pattern: String,
    pub dir: String,
}

impl Default for DownloadRule {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pattern: String::new(),
            dir: String::new(),
        }
    }
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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            always_ask_path: false,
            organize_by_platform: false,
            skip_existing: true,
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
