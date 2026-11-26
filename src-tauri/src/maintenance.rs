//! Diagnostics and queue import/export, added in v0.14.0.

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use std::path::Path;

use crate::{history, storage, AppState, recovery, follows};

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostics {
    pub data_dir: String,
    pub settings_file: String,
    pub output_dir: String,
    pub history_count: usize,
    pub follows_count: usize,
    pub recovery_count: usize,
    pub autostart: bool,
    pub close_to_tray: bool,
    pub proxy_configured: bool,
    pub cookies_configured: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueBackupEntry {
    pub url: String,
    #[serde(default = "default_mode")]
    pub download_mode: String,
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default)]
    pub format_id: Option<String>,
    #[serde(default)]
    pub playlist_items: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
}

fn default_mode() -> String { "auto".to_string() }
fn default_quality() -> String { "best".to_string() }

pub fn diagnostics(state: &AppState) -> Diagnostics {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    Diagnostics {
        data_dir: state.data_dir.to_string_lossy().into_owned(),
        settings_file: state.data_dir.join("settings.json").to_string_lossy().into_owned(),
        output_dir: settings.output_dir.clone(),
        history_count: history::list(&state.data_dir).len(),
        follows_count: follows::list(&state.data_dir).len(),
        recovery_count: recovery::list(&state.data_dir).len(),
        autostart: settings.autostart,
        close_to_tray: settings.close_to_tray,
        proxy_configured: !settings.proxy.trim().is_empty(),
        cookies_configured: !settings.cookie_file.trim().is_empty()
            || !settings.cookies_from_browser.trim().is_empty(),
    }
}

pub fn export_queue(state: &AppState, path: &Path) -> std::io::Result<()> {
    let entries: Vec<QueueBackupEntry> = state
        .queue
        .snapshot()
        .into_iter()
        .map(|info| QueueBackupEntry {
            url: info.url,
            download_mode: info.download_mode.unwrap_or_else(default_mode),
            quality: info.quality.unwrap_or_else(default_quality),
            format_id: info.format_id,
            playlist_items: info.playlist_items,
            output_dir: info.output_dir,
        })
        .collect();
    storage::write_json_atomic(path, &entries)
}

pub fn import_queue(app: &AppHandle, state: &AppState, path: &Path) -> Result<usize, String> {
    let entries: Vec<QueueBackupEntry> = storage::read_json(path)
        .map_err(|e| format!("Could not read queue file: {e}"))?;
    if entries.len() > 500 {
        return Err("This file has more than 500 links. Split it first.".to_string());
    }
    let mut added = 0usize;
    for entry in entries {
        let options = crate::models::StartOptions {
            url: entry.url.clone(),
            download_mode: crate::models::DownloadMode::parse(&entry.download_mode).as_str().to_string(),
            quality: entry.quality,
            format_id: entry.format_id,
            playlist_items: entry.playlist_items,
            output_dir: entry.output_dir,
        };
        if state.queue.start(app, &options.url, &options).is_ok() {
            added += 1;
        }
    }
    Ok(added)
}
