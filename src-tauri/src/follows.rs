//! Channel following: remembers channels, checks for new public uploads
//! through yt-dlp's flat playlist mode, and queues what has not been seen
//! yet. Baselines are recorded at follow time so the backlog never floods
//! the queue; only genuinely new uploads are fetched.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::models::{ChannelEntry, FollowChannel};
use crate::storage;
use crate::ytdlp;
use crate::AppState;

fn follows_path(data_dir: &Path) -> PathBuf {
    data_dir.join("follows.json")
}

pub fn list(data_dir: &Path) -> Vec<FollowChannel> {
    storage::read_json_or_default(&follows_path(data_dir))
}

fn save(data_dir: &Path, follows: &[FollowChannel]) -> std::io::Result<()> {
    storage::write_json_atomic(&follows_path(data_dir), &follows.to_vec())
}

pub fn add(data_dir: &Path, channel: FollowChannel) -> std::io::Result<()> {
    let mut follows = list(data_dir);
    follows.push(channel);
    save(data_dir, &follows)
}

pub fn remove(data_dir: &Path, id: &str) -> std::io::Result<bool> {
    let before = list(data_dir);
    let after: Vec<FollowChannel> = before.iter().filter(|f| f.id != id).cloned().collect();
    if after.len() == before.len() {
        return Ok(false);
    }
    save(data_dir, &after)?;
    Ok(true)
}

pub fn set_auto(data_dir: &Path, id: &str, auto_download: bool) -> std::io::Result<bool> {
    let mut follows = list(data_dir);
    let mut found = false;
    for follow in follows.iter_mut() {
        if follow.id == id {
            follow.auto_download = auto_download;
            found = true;
        }
    }
    if found {
        save(data_dir, &follows)?;
    }
    Ok(found)
}

/// YouTube channel home pages resolve to the uploads playlist. Any other
/// URL is passed to yt-dlp unchanged.
fn channel_target(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/').to_string();
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("youtube.com/@")
        || lower.contains("youtube.com/channel/")
        || lower.contains("youtube.com/c/")
        || lower.contains("youtube.com/user/")
    {
        if !lower.ends_with("/videos") && !lower.ends_with("/streams") && !lower.ends_with("/shorts") {
            return format!("{trimmed}/videos");
        }
    }
    trimmed
}

async fn fetch_flat_playlist(app: &AppHandle, target: &str) -> Result<Value, String> {
    let (settings, data_dir) = {
        let Some(state) = app.try_state::<AppState>() else {
            return Err("App is not ready yet.".to_string());
        };
        let settings = state.settings.lock().map_err(|_| "settings locked".to_string())?.clone();
        (settings, state.data_dir.clone())
    };
    let ytdlp_path = ytdlp::locate_ytdlp(settings.ytdlp_path.as_deref(), &data_dir)
        .ok_or_else(|| "The download engine is not installed. Install it in Settings, Engine.".to_string())?;

    let mut command = tokio::process::Command::new(&ytdlp_path);
    command
        .args([
            "--no-colors",
            "--flat-playlist",
            "--dump-single-json",
            "--no-warnings",
            "--playlist-end",
            "15",
        ])
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("PYTHONUTF8", "1")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .kill_on_drop(true);
    ytdlp::hide_console_tokio(&mut command);

    let output = tokio::time::timeout(std::time::Duration::from_secs(45), command.output())
        .await
        .map_err(|_| "Timed out while reading this channel.".to_string())?
        .map_err(|e| format!("Could not run yt-dlp: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("yt-dlp failed");
        return Err(tail.trim().to_string());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("Could not parse the engine response: {e}"))
}

fn entries_from_json(value: &Value) -> (String, Vec<ChannelEntry>) {
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("Channel")
        .to_string();
    let entries = value
        .get("entries")
        .and_then(|e| e.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|entry| {
                    let id = entry.get("id").and_then(|v| v.as_str())?.to_string();
                    let title = entry
                        .get("title")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .unwrap_or("(untitled)")
                        .to_string();
                    let url = entry
                        .get("url")
                        .and_then(|v| v.as_str())
                        .or_else(|| entry.get("webpage_url").and_then(|v| v.as_str()))
                        .map(String::from)
                        .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));
                    Some(ChannelEntry { id, title, url })
                })
                .collect()
        })
        .unwrap_or_default();
    (title, entries)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Resolve a channel URL and record it as followed. The backlog is baselined,
/// not downloaded.
pub async fn follow_add(app: &AppHandle, url: &str) -> Result<FollowChannel, String> {
    let target = channel_target(url);
    let value = fetch_flat_playlist(app, &target).await?;
    let (title, entries) = entries_from_json(&value);
    if entries.is_empty() {
        return Err("No public uploads found on this channel.".to_string());
    }
    let data_dir = app
        .try_state::<AppState>()
        .map(|state| state.data_dir.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    let follow = FollowChannel {
        id: uuid::Uuid::new_v4().to_string(),
        url: url.trim().to_string(),
        title,
        added_at: now_secs(),
        last_checked_at: Some(now_secs()),
        auto_download: false,
        seen_ids: entries.into_iter().map(|e| e.id).collect(),
    };
    add(&data_dir, follow.clone()).map_err(|e| e.to_string())?;
    Ok(follow)
}

/// Check the channel for uploads not seen yet and enqueue them.
/// Returns how many were enqueued.
pub async fn check_and_enqueue(app: &AppHandle, id: &str) -> Result<usize, String> {
    let data_dir = app
        .try_state::<AppState>()
        .map(|state| state.data_dir.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    let follow = list(&data_dir)
        .into_iter()
        .find(|f| f.id == id)
        .ok_or_else(|| "Follow not found.".to_string())?;

    let target = channel_target(&follow.url);
    let value = fetch_flat_playlist(app, &target).await?;
    let (_title, entries) = entries_from_json(&value);

    let seen: HashSet<String> = follow.seen_ids.iter().cloned().collect();
    let quality = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.lock().ok().map(|s| s.video_quality.clone()))
        .unwrap_or_else(|| "best".to_string());

    let mut enqueued = 0usize;
    let mut new_seen = follow.seen_ids.clone();
    for entry in entries {
        if seen.contains(&entry.id) {
            continue;
        }
        let options = crate::models::StartOptions {
            url: entry.url.clone(),
            download_mode: "auto".to_string(),
            quality: quality.clone(),
            format_id: None,
            playlist_items: None,
            output_dir: None,
        };
        if let Some(state) = app.try_state::<AppState>() {
            if state.queue.start(app, &entry.url, &options).is_ok() {
                enqueued += 1;
            }
        }
        new_seen.push(entry.id.clone());
    }

    // Bound the baseline file; the newest entries stay.
    let cap = 200usize;
    if new_seen.len() > cap {
        new_seen = new_seen.split_off(new_seen.len() - cap);
    }
    let mut follow = follow;
    follow.seen_ids = new_seen;
    follow.last_checked_at = Some(now_secs());
    let updated = list(&data_dir)
        .into_iter()
        .map(|f| if f.id == follow.id { follow.clone() } else { f })
        .collect::<Vec<_>>();
    let _ = save(&data_dir, &updated);
    Ok(enqueued)
}

/// Background poll for channels with auto-download enabled.
pub async fn poll_auto(app: AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(90)).await;
    loop {
        let data_dir = app.try_state::<AppState>().map(|s| s.data_dir.clone());
        if let Some(dir) = data_dir {
            for follow in list(&dir) {
                if follow.auto_download {
                    let _ = check_and_enqueue(&app, &follow.id).await;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
    }
}
