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
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub speed_limit_mbps: u32,
    #[serde(default)]
    pub start_at: Option<i64>,
    #[serde(default)]
    pub expected_sha256: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub after_id: Option<u64>,
    #[serde(default)]
    pub preset_id: Option<String>,
    #[serde(default)]
    pub direct: bool,
}

fn default_mode() -> String { "auto".to_string() }
fn default_quality() -> String { "best".to_string() }

pub fn diagnostics(state: &AppState) -> Diagnostics {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    diagnostics_from(&state.data_dir, &settings)
}

/// Reads the state files without holding an AppHandle, so the caller can run
/// it in `spawn_blocking` and keep the async worker free.
pub fn diagnostics_from(data_dir: &std::path::Path, settings: &crate::settings::AppSettings) -> Diagnostics {
    Diagnostics {
        data_dir: data_dir.to_string_lossy().into_owned(),
        settings_file: data_dir.join("settings.json").to_string_lossy().into_owned(),
        output_dir: settings.output_dir.clone(),
        history_count: history::list(data_dir).len(),
        follows_count: follows::list(data_dir).len(),
        recovery_count: recovery::list(data_dir).len(),
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
            priority: info.priority,
            speed_limit_mbps: info.speed_limit_mbps,
            start_at: info.scheduled_at,
            expected_sha256: None,
            group_id: info.group_id,
            after_id: info.after_id,
            preset_id: None,
            direct: false,
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
    let mut skipped = 0usize;
    let mut recovery_entries = Vec::new();
    for entry in entries {
        let options = crate::models::StartOptions {
            url: entry.url.clone(),
            download_mode: crate::models::DownloadMode::parse(&entry.download_mode).as_str().to_string(),
            quality: entry.quality,
            format_id: entry.format_id,
            playlist_items: entry.playlist_items,
            output_dir: entry.output_dir,
            start_at: entry.start_at,
            priority: entry.priority,
            speed_limit_mbps: entry.speed_limit_mbps,
            expected_sha256: entry.expected_sha256,
            group_id: entry.group_id,
            after_id: entry.after_id,
            preset_id: entry.preset_id,
            direct: entry.direct,
        };
        match state.queue.start_batch_item(app, &options) {
            Ok((_, true, recovery)) => {
                added += 1;
                if let Some(entry) = recovery {
                    recovery_entries.push(entry);
                }
            }
            Ok((_, false, _)) => skipped += 1,
            Err(_) => skipped += 1,
        }
    }
    // One recovery write, one emit and one scheduler pass for the whole file.
    state.queue.finish_batch(app, recovery_entries);
    if skipped > 0 {
        tracing::warn!("queue import skipped {skipped} invalid or duplicate links");
    }
    Ok(added)
}
