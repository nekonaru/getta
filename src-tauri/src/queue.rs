//! The download queue: admission, execution, progress, pause/resume, retry.
//!
//! Every state mutation goes through the inner mutex and ends with a full
//! snapshot emit to the frontend. Progress lines from yt-dlp are throttled
//! to 4 Hz; structural changes emit immediately.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Timelike;
use sha2::Digest;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::models::{Analysis, DownloadMode, QueueItemInfo, QueueStatus, RecoveryEntry};
use crate::platform;
use crate::recovery;
use crate::ytdlp;
use crate::{AppState, history};

use tauri::{Emitter, Manager};

static ID_SEED: AtomicU64 = AtomicU64::new(0);
static GEN_SEED: AtomicU64 = AtomicU64::new(0);

/// Monotonic token for one attempt of a queue item. A new attempt
/// invalidates the old background job so a killed process cannot finalize
/// a download that has been requeued.
fn next_generation() -> u64 {
    GEN_SEED.fetch_add(1, Ordering::Relaxed) + 1
}

fn next_id() -> u64 {
    let seed = ID_SEED.fetch_add(1, Ordering::Relaxed);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    now.saturating_mul(1000).saturating_add(seed % 1000)
}

#[derive(Clone)]
struct QueueItem {
    id: u64,
    url: String,
    platform: String,
    title: String,
    download_mode: DownloadMode,
    quality: String,
    format_id: Option<String>,
    status: QueueStatus,
    percent: f64,
    speed_bps: u64,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    eta_seconds: Option<u64>,
    phase: Option<String>,
    file_path: Option<String>,
    file_count: Option<u32>,
    thumbnail_url: Option<String>,
    author: Option<String>,
    duration_seconds: Option<f64>,
    error_message: Option<String>,
    retryable: bool,
    retry_count: u32,
    cancelled: bool,
    pid: Option<u32>,
    generation: u64,
    next_attempt_at: Option<i64>,
    file_paths: Vec<String>,
    started_at: Option<i64>,
    /// Unix seconds. None means start as soon as a slot is free.
    not_before: Option<i64>,
    output_dir: Option<String>,
    playlist_items: Option<String>,
    priority: i32,
    speed_limit_mbps: u32,
    expected_sha256: Option<String>,
    group_id: Option<String>,
    after_id: Option<u64>,
    preset_id: Option<String>,
    direct: bool,
    post_done: bool,
}

impl QueueItem {
    fn info(&self) -> QueueItemInfo {
        QueueItemInfo {
            id: self.id,
            url: self.url.clone(),
            title: self.title.clone(),
            platform: self.platform.clone(),
            status: self.status.as_str().to_string(),
            error_message: self.error_message.clone(),
            retryable: self.retryable,
            percent: self.percent,
            speed_bps: self.speed_bps,
            downloaded_bytes: self.downloaded_bytes,
            total_bytes: self.total_bytes,
            eta_seconds: self.eta_seconds,
            phase: self.phase.clone(),
            file_path: self.file_path.clone(),
            file_count: self.file_count,
            thumbnail_url: self.thumbnail_url.clone(),
            author: self.author.clone(),
            duration_seconds: self.duration_seconds,
            quality: Some(self.quality.clone()),
            download_mode: Some(self.download_mode.as_str().to_string()),
            format_id: self.format_id.clone(),
            playlist_items: self.playlist_items.clone(),
            output_dir: self.output_dir.clone(),
            scheduled_at: self.not_before,
            priority: self.priority,
            speed_limit_mbps: self.speed_limit_mbps,
            group_id: self.group_id.clone(),
            after_id: self.after_id,
        }
    }
}

struct QueueInner {
    items: Vec<QueueItem>,
}

pub struct QueueManager {
    inner: Mutex<QueueInner>,
    data_dir: PathBuf,
}

pub(crate) fn validate_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let path_part = trimmed.split(['?', '#']).next().unwrap_or(trimmed);
        if path_part.to_ascii_lowercase().ends_with(".torrent") {
            return Err("Torrents and magnet links arrive in a later release.".to_string());
        }
        return Ok(trimmed.to_string());
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("ytsearch") {
        // App-constructed YouTube search (e.g. "ytsearch5:query"). Passed to
        // yt-dlp as a single argument; it is never run through a shell.
        return Ok(trimmed.to_string());
    }
    if trimmed.starts_with("magnet:") || lower.ends_with(".torrent") {
        return Err("Torrents and magnet links arrive in a later release.".to_string());
    }
    Err("Only http(s) links are supported right now.".to_string())
}

impl QueueManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            inner: Mutex::new(QueueInner { items: Vec::new() }),
            data_dir,
        }
    }

    pub fn snapshot(&self) -> Vec<QueueItemInfo> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).items.iter().map(QueueItem::info).collect()
    }

    /// Returns the item id and whether a new item was actually inserted.
    pub fn start_counted(&self, app: &tauri::AppHandle, url: &str, options: &crate::models::StartOptions) -> Result<(u64, bool), String> {
        let trimmed = validate_url(url)?;
        let now = now_secs();
        let scheduled = options.start_at.filter(|at| *at > now);
        // Site profiles can override quality and mode for matching links.
        let profile = app
            .try_state::<AppState>()
            .and_then(|state| {
                state
                    .settings
                    .lock()
                    .ok()
                    .and_then(|settings| crate::settings::resolve_profile(&settings, &trimmed).cloned())
            });
        let quality = profile
            .as_ref()
            .filter(|p| !p.quality.trim().is_empty())
            .map(|p| p.quality.trim().to_string())
            .unwrap_or_else(|| {
                if options.quality.trim().is_empty() {
                    "best".to_string()
                } else {
                    options.quality.trim().to_string()
                }
            });
        let mode = profile
            .as_ref()
            .filter(|p| !p.mode.trim().is_empty())
            .map(|p| DownloadMode::parse(&p.mode))
            .unwrap_or_else(|| DownloadMode::parse(&options.download_mode));
        let mode_text = mode.as_str().to_string();
        let trimmed_lc = trimmed.to_ascii_lowercase();
        let is_search = trimmed_lc.starts_with("ytsearch");
        let platform_name = if is_search {
            "YouTube Search".to_string()
        } else {
            let platform_info = platform::detect(&trimmed);
            if !platform_info.supported {
                return Err(format!("{} links are not supported yet.", platform_info.display_name));
            }
            platform_info.display_name
        };

        let item = QueueItem {
            id: next_id(),
            url: trimmed.to_string(),
            platform: platform_name,
            title: title_from_url(&trimmed),
            download_mode: mode,
            quality: quality.clone(),
            format_id: options.format_id.clone(),
            status: QueueStatus::Queued,
            percent: 0.0,
            speed_bps: 0,
            downloaded_bytes: 0,
            total_bytes: None,
            eta_seconds: None,
            phase: None,
            file_path: None,
            file_count: None,
            thumbnail_url: None,
            author: None,
            duration_seconds: None,
            error_message: None,
            retryable: false,
            retry_count: 0,
            cancelled: false,
            pid: None,
            generation: next_generation(),
            next_attempt_at: None,
            file_paths: Vec::new(),
            started_at: None,
            not_before: scheduled,
            output_dir: options.output_dir.clone().filter(|d| !d.trim().is_empty()),
            playlist_items: options
                .playlist_items
                .clone()
                .filter(|s| !s.trim().is_empty())
                .filter(|s| s.chars().all(|c| c.is_ascii_digit() || matches!(c, ',' | '-' | ' '))),
            priority: options.priority,
            speed_limit_mbps: options.speed_limit_mbps,
            expected_sha256: options.expected_sha256.clone().filter(|s| !s.trim().is_empty()),
            group_id: options.group_id.clone().filter(|s| !s.trim().is_empty()),
            after_id: options.after_id,
            preset_id: options.preset_id.clone().filter(|s| !s.trim().is_empty()),
            direct: options.direct,
            post_done: false,
        };
        // Coalesce a double click or repeated enqueue of the same link while
        // it is still active or queued. Two jobs writing the same file is a
        // corruption risk, and --no-overwrites does not serialise them.
        if let Some(existing) = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .items
            .iter()
            .find(|i| {
                i.url == trimmed
                    && matches!(i.status, QueueStatus::Queued | QueueStatus::Active)
                    && i.download_mode == mode
                    && scheduled.is_none()
                    && i.not_before.is_none()
            })
            .map(|i| i.id)
        {
            return Ok((existing, false));
        }
        let item_title = item.title.clone();
        let id = item.id;
        {
            self.inner.lock().unwrap_or_else(|e| e.into_inner()).items.push(item);
        }
        // Remember it: if the process dies mid-download, launch can offer it back.
        let recovery_entry = RecoveryEntry {
            id,
            url: trimmed.to_string(),
            title: item_title,
            download_mode: mode_text,
            quality,
            format_id: options.format_id.clone(),
            playlist_items: options.playlist_items.clone(),
            output_dir: options.output_dir.clone(),
            start_at: scheduled,
        };
        let _ = recovery::push(&self.data_dir, recovery_entry);
        emit_queue(app);
        self.try_start_next(app);
        Ok((id, true))
    }

    pub fn start(&self, app: &tauri::AppHandle, url: &str, options: &crate::models::StartOptions) -> Result<u64, String> {
        self.start_counted(app, url, options).map(|(id, _)| id)
    }

    pub fn try_start_next(&self, app: &tauri::AppHandle) {
        let (max_concurrent, quiet, disk_guard) = app
            .try_state::<AppState>()
            .and_then(|state| {
                state.settings.lock().ok().map(|s| {
                    (
                        s.max_concurrent.max(1),
                        s.quiet_hours_enabled && in_quiet_hours(s.quiet_start_hour, s.quiet_end_hour),
                        s.disk_guard_gb,
                    )
                })
            })
            .unwrap_or((2, false, 0));
        if quiet {
            return;
        }
        if disk_guard > 0 {
            if let Some(state) = app.try_state::<AppState>() {
                let needed = disk_guard as u64 * 1024 * 1024 * 1024;
                let output = state
                    .settings
                    .lock()
                    .map(|s| std::path::PathBuf::from(&s.output_dir))
                    .unwrap_or_else(|_| state.data_dir.clone());
                let report = crate::features::health(&state.data_dir, &output, needed);
                if !report.enough_space {
                    return;
                }
            }
        }
        let mut to_spawn: Vec<(u64, u64)> = Vec::new();
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let running = inner.items.iter().filter(|i| i.status == QueueStatus::Active).count() as u32;
            let free = max_concurrent.saturating_sub(running);
            let now = now_secs();
            let finished: std::collections::HashSet<u64> = inner
                .items
                .iter()
                .filter(|i| matches!(i.status, QueueStatus::Done | QueueStatus::Error))
                .map(|i| i.id)
                .collect();
            let mut queued: Vec<(i32, u64, u64)> = inner
                .items
                .iter()
                .filter(|i| i.status == QueueStatus::Queued)
                .filter(|i| i.next_attempt_at.map_or(true, |at| at <= now))
                .filter(|i| i.not_before.map_or(true, |at| at <= now))
                .filter(|i| i.after_id.map_or(true, |id| finished.contains(&id)))
                .map(|i| (i.priority, i.id, i.generation))
                .collect();
            // Higher priority first, stable within the same priority so the
            // user's manual order survives.
            queued.sort_by(|a, b| b.0.cmp(&a.0));
            for (_, id, generation) in queued.into_iter().take(free as usize) {
                if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                    item.status = QueueStatus::Active;
                    item.phase = Some("starting".to_string());
                    item.started_at = Some(now);
                    item.pid = None;
                    to_spawn.push((id, generation));
                }
            }
        }
        for (id, generation) in to_spawn {
            spawn_job(app.clone(), id, generation);
        }
        sync_prevent_sleep(app);
    }

    /// Turn a scheduled item into an ordinary queued item.
    pub fn clear_schedule(&self, app: &tauri::AppHandle, id: u64) {
        let changed = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                if item.status == QueueStatus::Queued && item.not_before.is_some() {
                    item.not_before = None;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        if changed {
            emit_queue(app);
            self.try_start_next(app);
        }
    }

    pub fn tick_scheduled(&self, app: &tauri::AppHandle) {
        let now = now_secs();
        let due = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.items.iter().any(|i| {
                i.status == QueueStatus::Queued && i.not_before.map_or(false, |at| at <= now)
            })
        };
        if due {
            self.try_start_next(app);
            emit_queue(app);
        }
    }

    pub fn pause(&self, app: &tauri::AppHandle, id: u64) -> Result<(), String> {
        let pid = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let item = inner.items.iter().find(|i| i.id == id).ok_or("item not found")?;
            if item.status != QueueStatus::Active {
                return Err("Only running downloads can be paused.".to_string());
            }
            item.pid
        };
        if let Some(pid) = pid {
            suspend_process(pid, true).map_err(|e| format!("Could not pause the download: {e}"))?;
        }
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                item.status = QueueStatus::Paused;
                item.speed_bps = 0;
                item.phase = Some("paused".to_string());
            }
        }
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
        Ok(())
    }

    pub fn resume(&self, app: &tauri::AppHandle, id: u64) -> Result<(), String> {
        let (pid, active_now, max_concurrent) = {
            let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let item = inner.items.iter().find(|i| i.id == id).ok_or("item not found")?;
            if item.status != QueueStatus::Paused {
                return Err("Only paused downloads can be resumed.".to_string());
            }
            let active = inner.items.iter().filter(|i| i.status == QueueStatus::Active).count() as u32;
            let max = app
                .try_state::<AppState>()
                .and_then(|state| state.settings.lock().ok().map(|s| s.max_concurrent))
                .unwrap_or(2)
                .max(1);
            (item.pid, active, max)
        };

        // Pause freezing frees the concurrency slot; try_start_next fills it.
        // So on resume: continue in place only if a slot is free; otherwise kill
        // the frozen process and re-queue it (yt-dlp resumes from .part files).
        if pid.is_some() && active_now < max_concurrent {
            if let Some(pid) = pid {
                suspend_process(pid, false).map_err(|e| format!("Could not resume the download: {e}"))?;
            }
            {
                let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                    item.status = QueueStatus::Active;
                    item.phase = Some("downloading".to_string());
                }
            }
            emit_queue(app);
            sync_prevent_sleep(app);
        } else {
            if let Some(pid) = pid {
                kill_process(pid);
            }
            {
                let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                    // yt-dlp picks up from the .part file in the output dir.
                    item.pid = None;
                    // Invalidate the suspended job: it will be killed below.
                    item.generation = next_generation();
                    item.status = QueueStatus::Queued;
                    item.phase = Some("resuming".to_string());
                    item.speed_bps = 0;
                }
            }
            emit_queue(app);
            self.try_start_next(app);
        }
        Ok(())
    }

    pub fn cancel(&self, app: &tauri::AppHandle, id: u64) {
        let (pid, was_running) = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let Some(index) = inner.items.iter().position(|i| i.id == id) else {
                return;
            };
            inner.items[index].cancelled = true;
            let running = matches!(inner.items[index].status, QueueStatus::Active | QueueStatus::Paused);
            let pid = if running { inner.items[index].pid } else { None };
            inner.items.remove(index);
            (pid, running)
        };
        if was_running {
            if let Some(pid) = pid {
                kill_process(pid);
            }
        }
        let _ = recovery::remove(&self.data_dir, id);
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
    }

    pub fn retry(&self, app: &tauri::AppHandle, id: u64) {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                if item.status != QueueStatus::Error {
                    return;
                }
                item.generation = next_generation();
                item.next_attempt_at = None;
                item.status = QueueStatus::Queued;
                item.error_message = None;
                item.retryable = false;
                item.percent = 0.0;
                item.speed_bps = 0;
                item.downloaded_bytes = 0;
                item.total_bytes = None;
                item.eta_seconds = None;
                item.phase = None;
                item.cancelled = false;
                item.retry_count = 0;
            }
        }
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
    }

    pub fn remove(&self, app: &tauri::AppHandle, id: u64, delete_file: bool) {
        let (pid, running, paths) = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let Some(index) = inner.items.iter().position(|i| i.id == id) else {
                return;
            };
            let item = inner.items.remove(index);
            let running = matches!(item.status, QueueStatus::Active | QueueStatus::Paused);
            let pid = if running { item.pid } else { None };
            let mut paths = item.file_paths;
            if let Some(path) = item.file_path {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
            (pid, running, if delete_file { paths } else { Vec::new() })
        };
        if running {
            if let Some(pid) = pid {
                kill_process(pid);
            }
        }
        for path in paths {
            let _ = std::fs::remove_file(&path);
        }
        let _ = recovery::remove(&self.data_dir, id);
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
    }

    pub fn move_item(&self, app: &tauri::AppHandle, id: u64, delta: i32) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let len = inner.items.len();
            let index = inner.items.iter().position(|i| i.id == id).ok_or("item not found")?;
            if inner.items[index].status != QueueStatus::Queued {
                return Err("Only queued items can be reordered".to_string());
            }
            if delta < 0 {
                let mut i = index;
                while i > 0 {
                    i -= 1;
                    if inner.items[i].status == QueueStatus::Queued {
                        inner.items.swap(i, index);
                        break;
                    }
                }
            } else {
                let mut i = index;
                while i + 1 < len {
                    i += 1;
                    if inner.items[i].status == QueueStatus::Queued {
                        inner.items.swap(i, index);
                        break;
                    }
                }
            }
        }
        emit_queue(app);
        Ok(())
    }

    pub fn move_item_to(&self, app: &tauri::AppHandle, id: u64, target_index: u32) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            let queued: Vec<QueueItem> = inner
                .items
                .iter()
                .filter(|i| i.status == QueueStatus::Queued)
                .cloned()
                .collect();
            let from = queued
                .iter()
                .position(|i| i.id == id)
                .ok_or("item not found or not queued")?;
            if queued.is_empty() {
                return Ok(());
            }
            let target = (target_index as usize).min(queued.len() - 1);
            if from == target {
                return Ok(());
            }
            let mut reordered = queued;
            let item = reordered.remove(from);
            reordered.insert(target, item);
            let mut slot = 0;
            for entry in inner.items.iter_mut().filter(|i| i.status == QueueStatus::Queued) {
                *entry = reordered[slot].clone();
                slot += 1;
            }
        }
        emit_queue(app);
        Ok(())
    }

    /// Requeue every failed item. Returns how many were requeued.
    pub fn retry_all_failed(&self, app: &tauri::AppHandle) -> usize {
        let mut count = 0usize;
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            for item in inner.items.iter_mut() {
                if item.status != QueueStatus::Error {
                    continue;
                }
                count += 1;
                item.generation = next_generation();
                item.next_attempt_at = None;
                item.status = QueueStatus::Queued;
                item.error_message = None;
                item.retryable = false;
                item.retry_count = 0;
                item.percent = 0.0;
                item.speed_bps = 0;
                item.downloaded_bytes = 0;
                item.total_bytes = None;
                item.eta_seconds = None;
                item.phase = None;
                item.cancelled = false;
            }
        }
        emit_queue(app);
        self.try_start_next(app);
        count
    }

    pub fn clear_finished(&self, app: &tauri::AppHandle) {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.items.retain(|i| !matches!(i.status, QueueStatus::Done | QueueStatus::Error));
        }
        emit_queue(app);
        sync_prevent_sleep(app);
    }

    /// Internal test hook: read a consistent item view.
    #[doc(hidden)]
    #[allow(dead_code)]
    pub fn items_len(&self) -> usize {
        self.inner.lock().map(|inner| inner.items.len()).unwrap_or(0)
    }
}

fn pause_all_impl(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<AppState>() else {
        return Ok(());
    };
    // Mark every active item paused under one lock. Pausing one by one would
    // free a concurrency slot and let try_start_next spawn a queued job.
    let pids: Vec<u32> = {
        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut pids = Vec::new();
        for item in inner.items.iter_mut() {
            if item.status != QueueStatus::Active {
                continue;
            }
            if let Some(pid) = item.pid {
                pids.push(pid);
            }
            item.status = QueueStatus::Paused;
            item.speed_bps = 0;
            item.phase = Some("paused".to_string());
        }
        pids
    };
    for pid in pids {
        let _ = suspend_process(pid, true);
    }
    emit_queue(app);
    sync_prevent_sleep(app);
    Ok(())
}

pub fn pause_all(app: &tauri::AppHandle) -> Result<(), String> {
    pause_all_impl(app)
}

pub fn resume_all(app: &tauri::AppHandle) -> Result<(), String> {
    let ids: Vec<u64> = app
        .try_state::<AppState>()
        .map(|state| {
            state
                .queue
                .snapshot()
                .into_iter()
                .filter(|i| i.status == "paused")
                .map(|i| i.id)
                .collect()
        })
        .unwrap_or_default();
    if let Some(state) = app.try_state::<AppState>() {
        for id in ids {
            let _ = state.queue.resume(app, id);
        }
    }
    Ok(())
}

/* ========== job execution ========== */

/// Keep the machine awake only while a download is actually running and
/// the user asked us to. Drops the guard as soon as the queue drains.
fn sync_prevent_sleep(app: &tauri::AppHandle) {
    let should = app
        .try_state::<AppState>()
        .map(|state| {
            let enabled = state.settings.lock().map(|s| s.prevent_sleep).unwrap_or(false);
            let active = state.queue.snapshot().into_iter().any(|i| i.status == "active");
            enabled && active
        })
        .unwrap_or(false);

    static GUARD: std::sync::OnceLock<std::sync::Mutex<Option<keepawake::KeepAwake>>> = std::sync::OnceLock::new();
    let slot = GUARD.get_or_init(|| std::sync::Mutex::new(None));
    let mut guard = match slot.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if should {
        if guard.is_none() {
            match keepawake::Builder::default()
                .idle(true)
                .reason("Getta is downloading")
                .app_name("Getta")
                .app_reverse_domain("com.getta.app")
                .create()
            {
                Ok(created) => *guard = Some(created),
                Err(e) => tracing::warn!("prevent_sleep unavailable: {e}"),
            }
        }
    } else {
        *guard = None;
    }
}

fn emit_queue(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let snapshot = state.queue.snapshot();
    if let Some(tray) = app.tray_by_id("main-tray") {
        let active = snapshot.iter().filter(|i| i.status == "active").count();
        let tooltip = if active == 0 {
            "Getta".to_string()
        } else {
            format!("Getta · {active} downloading")
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }
    let _ = app.emit("queue-changed", snapshot);
}

fn title_from_url(url: &str) -> String {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("ytsearch") {
        let query = url.splitn(2, ':').nth(1).unwrap_or("search").trim();
        let query = if query.is_empty() { "search" } else { query };
        return sanitize_filename::sanitize(query.chars().take(80).collect::<String>());
    }
    if let Ok(parsed) = url::Url::parse(url) {
        let host = parsed.host_str().unwrap_or("download").to_string();
        let tail = parsed
            .path_segments()
            .and_then(|mut segs| segs.next_back())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        return match tail {
            Some(t) if t.len() > 3 => sanitize_filename::sanitize(t.chars().take(80).collect::<String>()),
            _ => host,
        };
    }
    url.chars().take(80).collect()
}

/// Fire an opt-in webhook for a queue event. The request is spawned and
/// never blocks a download. Failures are ignored by design.
fn notify_webhook(app: &tauri::AppHandle, event: &str, title: &str, url: &str, error: Option<&str>) {
    let (endpoint, events, enabled) = app
        .try_state::<AppState>()
        .and_then(|state| {
            state.settings.lock().ok().map(|s| {
                (s.webhook_url.trim().to_string(), s.webhook_events.clone(), !s.webhook_url.trim().is_empty())
            })
        })
        .unwrap_or_default();
    if !enabled || !events.split(',').any(|e| e.trim() == event) {
        return;
    }
    let payload = serde_json::json!({
        "event": event,
        "title": title,
        "url": url,
        "error": error,
        "at": now_secs(),
    });
    tauri::async_runtime::spawn(async move {
        let _ = reqwest::Client::new().post(&endpoint).json(&payload).send().await;
    });
}

fn in_quiet_hours(start: u32, end: u32) -> bool {
    let hour = chrono::Local::now().hour();
    if start == end {
        return false;
    }
    if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

pub(crate) fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Cookie and proxy arguments shared by downloads, analysis and follow checks.
/// The same login context must be used everywhere or previews fail while
/// downloads succeed.
pub(crate) fn metadata_args(settings: &crate::settings::AppSettings) -> Vec<String> {
    let mut args = Vec::new();
    let cookie_file = settings.cookie_file.trim();
    let cookie_browser = settings.cookies_from_browser.trim();
    // A cookies file takes precedence: Chrome 127+ blocks DPAPI access,
    // so --cookies-from-browser fails the whole download on Windows.
    if !cookie_file.is_empty() && std::path::PathBuf::from(cookie_file).is_file() {
        args.push("--cookies".to_string());
        args.push(cookie_file.to_string());
    } else if !cookie_browser.is_empty() {
        args.push("--cookies-from-browser".to_string());
        args.push(cookie_browser.to_string());
    }
    if !settings.proxy.trim().is_empty() {
        args.push("--proxy".to_string());
        args.push(settings.proxy.trim().to_string());
    }
    args
}

fn build_args(
    item_url: &str,
    mode: DownloadMode,
    quality: &str,
    format_id: Option<&str>,
    playlist_items: Option<&str>,
    settings: &crate::settings::AppSettings,
    output_dir: &str,
) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--newline".to_string(),
        // --print makes yt-dlp quiet by default, which disables its progress
        // bar. Force it back on so percent, speed, ETA and size reach the UI.
        "--progress".to_string(),
        "--no-colors".to_string(),
        "--print".to_string(),
        "after_move:GettaFile:%(filepath)s".to_string(),
    ];

    let template = if settings.organize_by_platform {
        format!("{output_dir}/%(extractor_key)s/{}", settings.filename_template)
    } else {
        format!("{output_dir}/{}", settings.filename_template)
    };
    args.push("-o".to_string());
    args.push(template);

    if let Some(items) = playlist_items.map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--playlist-items".to_string());
        args.push(items.to_string());
    }

    if settings.skip_existing {
        args.push("--no-overwrites".to_string());
    }
    if settings.download_archive {
        // yt-dlp records video ids and skips links already downloaded.
        let archive = crate::storage::data_dir().join("download-archive.txt");
        args.push("--download-archive".to_string());
        args.push(archive.to_string_lossy().into_owned());
    }

    match mode {
        DownloadMode::Audio => {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push("m4a".to_string());
        }
        DownloadMode::Subs => {
            // Subtitles only: no media is fetched, just the SRT/VTT tracks.
            args.push("--skip-download".to_string());
            args.push("--write-subs".to_string());
            if !settings.subtitle_lang.trim().is_empty() {
                args.push("--sub-langs".to_string());
                args.push(settings.subtitle_lang.clone());
            } else {
                args.push("--all-subs".to_string());
            }
        }
        DownloadMode::Auto | DownloadMode::Mute => {
            if let Some(fmt) = format_id.filter(|f| !f.trim().is_empty()) {
                args.push("-f".to_string());
                args.push(fmt.to_string());
            } else if mode == DownloadMode::Mute {
                args.push("-f".to_string());
                args.push(match quality {
                    "best" => "bv*".to_string(),
                    h => format!("bv*[height<={h}]/bv*"),
                });
            } else {
                args.push("-f".to_string());
                args.push(match quality {
                    "best" => "bv*+ba/b".to_string(),
                    h => format!("bv*[height<={h}]+ba/b[height<={h}]/bv*+ba/b"),
                });
            }
        }
    }

    if settings.embed_metadata {
        args.push("--embed-metadata".to_string());
    }
    if settings.embed_thumbnail {
        args.push("--embed-thumbnail".to_string());
    }
    if settings.write_subtitles && !settings.subtitle_lang.trim().is_empty() {
        args.push("--write-subs".to_string());
        args.push("--sub-langs".to_string());
        args.push(settings.subtitle_lang.clone());
    }
    if settings.sponsorblock {
        args.push("--sponsorblock-remove".to_string());
        args.push("sponsor,selfpromo,interaction".to_string());
    }
    if settings.split_by_chapters {
        args.push("--split-chapters".to_string());
    }
    if settings.live_from_start {
        args.push("--live-from-start".to_string());
    }
    if settings.speed_limit_mbps > 0 {
        args.push("--limit-rate".to_string());
        args.push(format!("{}M", settings.speed_limit_mbps));
    }
    if settings.concurrent_fragments > 0 {
        args.push("--concurrent-fragments".to_string());
        args.push(settings.concurrent_fragments.to_string());
    }
    args.extend(metadata_args(settings));

    let lower_url = item_url.to_ascii_lowercase();
    let single_video = lower_url.contains("/watch?") || lower_url.contains("youtu.be/");
    // A watch URL with &list= is still one video unless the user picked
    // playlist entries; forcing --yes-playlist downloads the whole list.
    if playlist_items.is_none() && single_video {
        args.push("--no-playlist".to_string());
    } else {
        args.push("--yes-playlist".to_string());
    }
    args.push(item_url.to_string());
    args
}

/// True only while this background job is still the current attempt.
fn is_current(app: &tauri::AppHandle, id: u64, generation: u64) -> bool {
    app.try_state::<AppState>()
        .map(|state| {
            state
                .queue
                .inner
                .lock()
                .map(|inner| {
                    inner
                        .items
                        .iter()
                        .find(|i| i.id == id)
                        .map_or(false, |i| i.generation == generation)
                })
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn spawn_job(app: tauri::AppHandle, id: u64, generation: u64) {
    tauri::async_runtime::spawn(async move {
        let direct = app
            .try_state::<AppState>()
            .and_then(|state| {
                state
                    .queue
                    .inner
                    .lock()
                    .ok()
                    .and_then(|inner| inner.items.iter().find(|i| i.id == id).map(|i| i.direct))
            })
            .unwrap_or(false);
        if direct {
            run_direct_job(app, id, generation).await;
        } else {
            run_job(app, id, generation).await;
        }
    });
}

/// Native http(s) downloader for direct file links. Streams to a `.part`
/// file, resumes an existing partial file with a Range request, and reports
/// progress through the same queue snapshot as yt-dlp jobs.
async fn run_direct_job(app: tauri::AppHandle, id: u64, generation: u64) {
    let (url, output_dir, limit) = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let Some(item) = state
            .queue
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .items
            .iter()
            .find(|i| i.id == id)
            .cloned()
        else {
            return;
        };
        let dir = item.output_dir.clone().unwrap_or_else(|| settings.output_dir.clone());
        (item.url, dir, if item.speed_limit_mbps > 0 { item.speed_limit_mbps } else { settings.speed_limit_mbps })
    };
    if std::fs::create_dir_all(&output_dir).is_err() {
        finalize_error(&app, id, "The download folder cannot be created.", false);
        return;
    }
    let name = crate::direct::filename_from_url(&url);
    let final_path = std::path::PathBuf::from(&output_dir).join(name);
    let part_path = final_path.with_extension(format!(
        "{}.part",
        final_path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    let existing = std::fs::metadata(&part_path).ok().map(|m| m.len()).unwrap_or(0);
    let mut request = reqwest::Client::new().get(&url);
    if existing > 0 {
        request = request.header("Range", format!("bytes={existing}-"));
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(e) => {
            finalize_maybe_retry(&app, id, &format!("Network error: {e}"));
            return;
        }
    };
    if !response.status().is_success() && response.status().as_u16() != 206 {
        finalize_error(
            &app,
            id,
            &format!("Server returned HTTP {}", response.status().as_u16()),
            false,
        );
        return;
    }
    let resumed = response.status().as_u16() == 206 && existing > 0;
    let total = response
        .content_length()
        .map(|n| if resumed { n + existing } else { n });
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&part_path)
    {
        Ok(file) => file,
        Err(e) => {
            finalize_error(&app, id, &format!("Cannot write file: {e}"), false);
            return;
        }
    };
    let mut stream = response.bytes_stream();
    let mut downloaded = if resumed { existing } else { 0u64 };
    let mut last_emit = std::time::Instant::now();
    while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
        if !is_current(&app, id, generation) {
            return;
        }
        let Ok(bytes) = chunk else {
            finalize_maybe_retry(&app, id, "The connection was interrupted.");
            return;
        };
        if std::io::Write::write_all(&mut file, &bytes).is_err() {
            finalize_error(&app, id, "Could not write the downloaded file.", false);
            return;
        }
        downloaded = downloaded.saturating_add(bytes.len() as u64);
        if limit > 0 {
            let delay = Duration::from_millis((bytes.len() as f64 / (limit as f64 * 1_048_576.0) * 1000.0).ceil() as u64);
            if delay > Duration::from_millis(1) {
                tokio::time::sleep(delay).await;
            }
        }
        if last_emit.elapsed() >= Duration::from_millis(250) {
            last_emit = std::time::Instant::now();
            if let Some(state) = app.try_state::<AppState>() {
                let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                    item.downloaded_bytes = downloaded;
                    item.total_bytes = total;
                    item.percent = total
                        .map(|t| if t > 0 { ((downloaded as f64 / t as f64) * 100.0).min(100.0) } else { 0.0 })
                        .unwrap_or(0.0);
                    item.phase = Some("downloading".to_string());
                }
            }
            emit_queue(&app);
        }
    }
    drop(file);
    if std::fs::rename(&part_path, &final_path).is_err() {
        finalize_error(&app, id, "Could not finalize the downloaded file.", false);
        return;
    }
    let size = std::fs::metadata(&final_path).ok().map(|m| m.len());
    let (title, platform, thumbnail, quality) = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(item) = inner.items.iter_mut().find(|i| i.id == id) else { return };
        item.status = QueueStatus::Done;
        item.percent = 100.0;
        item.downloaded_bytes = size.unwrap_or(downloaded);
        item.total_bytes = size.or(total);
        item.speed_bps = 0;
        item.eta_seconds = None;
        item.phase = None;
        item.pid = None;
        (
            item.title.clone(),
            item.platform.clone(),
            item.thumbnail_url.clone(),
            item.quality.clone(),
        )
    };
    if let Some(state) = app.try_state::<AppState>() {
        let _ = app.asset_protocol_scope().allow_file(&final_path);
        history::push(
            &state.data_dir,
            crate::models::HistoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                url: url.clone(),
                platform,
                title: title.clone(),
                file_path: Some(final_path.to_string_lossy().into_owned()),
                file_size_bytes: size,
                success: true,
                error: None,
                completed_at: now_secs(),
                thumbnail_url: thumbnail,
                quality: Some(quality),
                download_mode: Some("auto".to_string()),
                tags: Vec::new(),
                rating: 0,
                note: String::new(),
                favorite: false,
                watch_later: false,
            },
        );
        let _ = recovery::remove(&state.data_dir, id);
        crate::features::audit(&state.data_dir, "direct_download", &url);
    }
    emit_queue(&app);
    notify_webhook(&app, "done", &title, &url, None);
    state_queue_start(&app);
}

fn state_queue_start(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.try_start_next(app);
    }
}

async fn run_job(app: tauri::AppHandle, id: u64, generation: u64) {
    let (output_dir, args, ytdlp_path) = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let mut settings = state.settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let Some(item) = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner()).items.iter().find(|i| i.id == id).cloned() else {
            return;
        };
        // A per-item cap overrides the global one without mutating settings.
        if item.speed_limit_mbps > 0 {
            settings.speed_limit_mbps = item.speed_limit_mbps;
        }
        // Rules redirect the folder only when the user did not pick one for this item.
        let output_dir = item
            .output_dir
            .clone()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| crate::settings::resolve_download_dir(&settings, &item.url, &settings.output_dir));
        let ytdlp_path = ytdlp::locate_ytdlp(settings.ytdlp_path.as_deref(), &state.data_dir);
        let args = build_args(&item.url, item.download_mode, &item.quality, item.format_id.as_deref(), item.playlist_items.as_deref(), &settings, &output_dir);
        (output_dir, args, ytdlp_path)
    };

    if std::fs::create_dir_all(&output_dir).is_err() {
        if is_current(&app, id, generation) {
            finalize_error(&app, id, "The download folder cannot be created.", false);
        }
        return;
    }
    let Some(ytdlp_path) = ytdlp_path else {
        if is_current(&app, id, generation) {
            finalize_error(&app, id, "The download engine is not installed. Install it in Settings, Engine.", false);
        }
        return;
    };

    let mut command = Command::new(&ytdlp_path);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PYTHONUTF8", "1")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .kill_on_drop(true);
    ytdlp::hide_console_tokio(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            if is_current(&app, id, generation) {
                finalize_error(&app, id, &format!("Could not start yt-dlp: {e}"), false);
            }
            return;
        }
    };

    {
        if let Some(state) = app.try_state::<AppState>() {
            let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                if item.generation != generation {
                    return;
                }
                item.pid = child.id();
                item.phase = Some("fetching info".to_string());
            }
        }
    }

    let stderr = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        let mut tail: Vec<String> = Vec::new();
        if let Some(stream) = stderr {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tail.push(line);
                if tail.len() > 60 {
                    tail.remove(0);
                }
            }
        }
        tail
    });

    let mut last_emit = Instant::now();
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        let mut structural_emit = false;
        while let Ok(Some(line)) = lines.next_line().await {
            match ytdlp::parse_line(&line) {
                ytdlp::LineEvent::Progress { percent, total_bytes, downloaded_bytes, speed_bps, eta_seconds } => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            if item.generation != generation || item.cancelled {
                                continue;
                            }
                            item.percent = percent;
                            item.total_bytes = total_bytes.or(item.total_bytes);
                            if let Some(bytes) = downloaded_bytes {
                                item.downloaded_bytes = bytes.max(item.downloaded_bytes);
                            }
                            item.speed_bps = speed_bps.unwrap_or(0);
                            item.eta_seconds = eta_seconds;
                            item.phase = Some("downloading".to_string());
                            if let Some(total) = item.total_bytes {
                                item.downloaded_bytes = ((percent / 100.0) * total as f64) as u64;
                            }
                        }
                    }
                    structural_emit = false;
                }
                ytdlp::LineEvent::Bytes { downloaded_bytes, speed_bps, eta_seconds } => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            if item.generation != generation || item.cancelled {
                                structural_emit = false;
                                continue;
                            }
                            item.downloaded_bytes = downloaded_bytes.max(item.downloaded_bytes);
                            item.speed_bps = speed_bps.unwrap_or(0);
                            item.eta_seconds = eta_seconds;
                        }
                    }
                    structural_emit = false;
                }
                ytdlp::LineEvent::Destination(path) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            if item.generation != generation {
                                structural_emit = false;
                                continue;
                            }
                            if !item.file_paths.contains(&path) {
                                item.file_paths.push(path.clone());
                            }
                            item.file_path = Some(path);
                            item.file_count = Some(item.file_count.unwrap_or(0) + 1);
                            item.phase = Some("saving".to_string());
                        }
                    }
                    structural_emit = true;
                }
                ytdlp::LineEvent::PostProcess(phase) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            if item.generation == generation {
                                item.phase = Some(phase);
                            }
                        }
                    }
                    structural_emit = true;
                }
                ytdlp::LineEvent::PlaylistItem(index) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            if item.generation == generation {
                                item.phase = Some(format!("item {index}"));
                            }
                        }
                    }
                    structural_emit = true;
                }
                ytdlp::LineEvent::Ignored => {}
            }
            if structural_emit || last_emit.elapsed() >= Duration::from_millis(250) {
                last_emit = Instant::now();
                emit_queue(&app);
            }
        }
    }

    let exit = child.wait().await;
    let stderr_tail = stderr_task.await.unwrap_or_default();
    if !is_current(&app, id, generation) {
        // Cancelled, removed, paused into a new attempt, or retried.
        emit_queue(&app);
        return;
    }

    match exit {
        Ok(status) if status.success() => {
            let (title, url, file_paths, fallback_path, file_size, platform, thumbnail_url, quality, mode, expected_sha256) = {
                let Some(state) = app.try_state::<AppState>() else { return };
                let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
                let Some(item) = inner.items.iter_mut().find(|i| i.id == id) else { return };
                if item.generation != generation {
                    return;
                }
                item.status = QueueStatus::Done;
                item.percent = 100.0;
                item.speed_bps = 0;
                item.eta_seconds = None;
                item.phase = None;
                item.pid = None;
                let size = item.file_path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len());
                item.downloaded_bytes = size
                    .or(item.total_bytes)
                    .unwrap_or(item.downloaded_bytes);
                let paths = item.file_paths.clone();
                let fallback_path = item.file_path.clone();
                (
                    item.title.clone(),
                    item.url.clone(),
                    paths,
                    fallback_path,
                    size,
                    item.platform.clone(),
                    item.thumbnail_url.clone(),
                    item.quality.clone(),
                    item.download_mode.as_str().to_string(),
                    item.expected_sha256.clone(),
                )
            };
            if let Some(expected) = expected_sha256.as_ref() {
                let target = file_paths.first().or(fallback_path.as_ref());
                let actual = target
                    .and_then(|path| std::fs::read(path).ok())
                    .map(|bytes| hex::encode(sha2::Sha256::digest(&bytes)));
                let matched = actual
                    .as_deref()
                    .map(|digest| digest.eq_ignore_ascii_case(expected.trim()))
                    .unwrap_or(false);
                if !matched {
                    let message = format!(
                        "Checksum mismatch. Expected {}, got {}.",
                        expected,
                        actual.unwrap_or_else(|| "no readable file".to_string())
                    );
                    finalize_error(&app, id, &message, false);
                    finalize_maybe_retry(&app, id, &message);
                    emit_queue(&app);
                    return;
                }
            }
            if let Some(state) = app.try_state::<AppState>() {
                // A playlist can produce several files. Record all of them so
                // "remove and delete file" cleans up the whole download.
                let paths = if file_paths.is_empty() {
                    fallback_path.into_iter().collect::<Vec<_>>()
                } else {
                    file_paths
                };
                if paths.is_empty() {
                    // Subtitle-only downloads produce no after_move print.
                    // Keep a history row even without a file path.
                    history::push(
                        &state.data_dir,
                        crate::models::HistoryEntry {
                            id: uuid::Uuid::new_v4().to_string(),
                            url: url.clone(),
                            platform: platform.clone(),
                            title: title.clone(),
                            file_path: None,
                            file_size_bytes: None,
                            success: true,
                            error: None,
                            completed_at: now_secs(),
                            thumbnail_url: thumbnail_url.clone(),
                            quality: Some(quality.clone()),
                            download_mode: Some(mode.clone()),
                            tags: Vec::new(),
                            rating: 0,
                            note: String::new(),
                            favorite: false,
                            watch_later: false,
                        },
                    );
                }
                for path in &paths {
                    if std::path::Path::new(path).is_file() {
                        let _ = app.asset_protocol_scope().allow_file(path);
                    }
                    let size = std::fs::metadata(path).ok().map(|m| m.len());
                    history::push(
                        &state.data_dir,
                        crate::models::HistoryEntry {
                            id: uuid::Uuid::new_v4().to_string(),
                            url: url.clone(),
                            platform: platform.clone(),
                            title: title.clone(),
                            file_path: Some(path.clone()),
                            file_size_bytes: size.or(file_size),
                            success: true,
                            error: None,
                            completed_at: now_secs(),
                            thumbnail_url: thumbnail_url.clone(),
                            quality: Some(quality.clone()),
                            download_mode: Some(mode.clone()),
                            tags: Vec::new(),
                            rating: 0,
                            note: String::new(),
                            favorite: false,
                            watch_later: false,
                        },
                    );
                }
                // Optional post-download move or copy. The history row still
                // points at the original path so removal stays predictable.
                let (post_action, post_dir) = state
                    .settings
                    .lock()
                    .map(|s| (s.post_action.clone(), s.post_action_dir.clone()))
                    .unwrap_or_default();
                if matches!(post_action.as_str(), "move" | "copy") && !post_dir.trim().is_empty() {
                    let base = std::path::PathBuf::from(post_dir.trim());
                    if std::fs::create_dir_all(&base).is_ok() {
                        for path in &paths {
                            let source = std::path::PathBuf::from(path);
                            let Some(name) = source.file_name() else { continue };
                            let target = base.join(name);
                            if target == source {
                                continue;
                            }
                            let result = if post_action == "move" {
                                std::fs::rename(&source, &target)
                            } else {
                                std::fs::copy(&source, &target).map(|_| ())
                            };
                            if result.is_ok() {
                                let _ = app.asset_protocol_scope().allow_file(&target);
                            }
                        }
                    }
                }
                let _ = recovery::remove(&state.data_dir, id);
            }
            emit_queue(&app);
            notify_webhook(&app, "done", &title, &url, None);
            if let Some(state) = app.try_state::<AppState>() {
                state.queue.try_start_next(&app);
            }
        }
        Ok(status) => {
            let tail = stderr_tail.join("\n");
            let message = if tail.trim().is_empty() {
                format!("yt-dlp exited with code {}", status.code().unwrap_or(-1))
            } else {
                tail.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("unknown error").to_string()
            };
            if is_current(&app, id, generation) {
                finalize_maybe_retry(&app, id, &message);
            }
        }
        Err(e) => {
            if is_current(&app, id, generation) {
                finalize_error(&app, id, &format!("yt-dlp failed to run: {e}"), false);
            }
        }
    }
}

fn finalize_maybe_retry(app: &tauri::AppHandle, id: u64, message: &str) {
    let (max_retries, base_delay, multiplier, storm_cap) = app
        .try_state::<AppState>()
        .and_then(|state| {
            state.settings.lock().ok().map(|s| {
                (
                    s.max_retries,
                    s.retry_delay_secs.max(1),
                    s.retry_multiplier.max(1.0),
                    s.retry_storm_cap.max(1),
                )
            })
        })
        .unwrap_or((3, 2, 2.0, 3));

    let (category, retryable) = ytdlp::classify_error(message);
    let retry_count = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.items.iter().find(|i| i.id == id) {
            Some(item) => item.retry_count,
            None => return,
        }
    };

    let can_retry = retryable && (category == "rate_limited" || category == "unknown")
        && retry_count < max_retries;

    if can_retry {
        let in_flight = app
            .try_state::<AppState>()
            .and_then(|state| {
                state.queue.inner.lock().ok().map(|inner| {
                    inner
                        .items
                        .iter()
                        .filter(|i| {
                            i.status == QueueStatus::Queued
                                && i.phase
                                    .as_deref()
                                    .map(|p| p.starts_with("retrying"))
                                    .unwrap_or(false)
                        })
                        .count() as u32
                })
            })
            .unwrap_or(0);
        if in_flight >= storm_cap {
            return;
        }
        let delay_secs = ((base_delay as f64) * multiplier.powi(retry_count.min(6) as i32))
            .round()
            .max(1.0)
            .min(600.0) as u64;
        if let Some(state) = app.try_state::<AppState>() {
            let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                item.generation = next_generation();
                item.next_attempt_at = Some(now_secs() + delay_secs as i64);
                item.status = QueueStatus::Queued;
                item.retry_count += 1;
                item.pid = None;
                item.phase = Some(format!("retrying in {delay_secs}s"));
                item.speed_bps = 0;
                item.eta_seconds = None;
            }
        }
        emit_queue(app);
        let retry_app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            if let Some(state) = retry_app.try_state::<AppState>() {
                if let Ok(mut inner) = state.queue.inner.lock() {
                    if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                        item.next_attempt_at = None;
                    }
                }
                state.queue.try_start_next(&retry_app);
            }
        });
        return;
    }

    finalize_error(app, id, message, retryable);
}

fn finalize_error(app: &tauri::AppHandle, id: u64, message: &str, retryable: bool) {
    let (url, title, platform, quality, mode) = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let mut inner = state.queue.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(item) = inner.items.iter_mut().find(|i| i.id == id) else { return };
        item.status = QueueStatus::Error;
        item.error_message = Some(message.to_string());
        item.retryable = retryable;
        item.pid = None;
        item.speed_bps = 0;
        item.phase = None;
        (
            item.url.clone(),
            item.title.clone(),
            item.platform.clone(),
            item.quality.clone(),
            item.download_mode.as_str().to_string(),
        )
    };
    notify_webhook(app, "error", &title, &url, Some(message));
    if let Some(state) = app.try_state::<AppState>() {
        history::push(
            &state.data_dir,
            crate::models::HistoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                url,
                platform,
                title,
                file_path: None,
                file_size_bytes: None,
                success: false,
                error: Some(message.to_string()),
                completed_at: now_secs(),
                thumbnail_url: None,
                quality: Some(quality),
                download_mode: Some(mode),
                tags: Vec::new(),
                rating: 0,
                note: String::new(),
                favorite: false,
                watch_later: false,
            },
        );
        let _ = recovery::remove(&state.data_dir, id);
    }
    emit_queue(app);
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.try_start_next(app);
    }
}

/* ========== process control ========== */

#[cfg(windows)]
mod ntdll {
    extern "C" {
        pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        pub fn CloseHandle(handle: isize) -> i32;
        pub fn NtSuspendProcess(handle: isize) -> i32;
        pub fn NtResumeProcess(handle: isize) -> i32;
        pub fn TerminateProcess(handle: isize, exit_code: u32) -> i32;
    }
}

#[cfg(windows)]
const PROCESS_SUSPEND_RESUME: u32 = 0x0800;
#[cfg(windows)]
const PROCESS_TERMINATE: u32 = 0x0001;

#[cfg(windows)]
fn suspend_process(pid: u32, suspend: bool) -> std::io::Result<()> {
    unsafe {
        let access = if suspend { PROCESS_SUSPEND_RESUME } else { PROCESS_SUSPEND_RESUME };
        let handle = ntdll::OpenProcess(access, 0, pid);
        if handle == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let result = if suspend {
            ntdll::NtSuspendProcess(handle)
        } else {
            ntdll::NtResumeProcess(handle)
        };
        ntdll::CloseHandle(handle);
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(result))
        }
    }
}

#[cfg(not(windows))]
fn suspend_process(pid: u32, suspend: bool) -> std::io::Result<()> {
    let signal = if suspend { libc::SIGSTOP } else { libc::SIGCONT };
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn kill_process(pid: u32) {
    unsafe {
        let handle = ntdll::OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle != 0 {
            ntdll::TerminateProcess(handle, 1);
            ntdll::CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
fn kill_process(pid: u32) {
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
}

/* ========== tools (local FFmpeg workbench) ========== */

#[derive(serde::Serialize, Clone)]
pub struct ToolProgressEvent {
    pub tool: String,
    pub percent: f64,
}

fn unique_output(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out").to_string();
    let ext = path.extension().and_then(|s| s.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
    for n in 1..100 {
        let candidate = path.with_file_name(format!("{stem}-{n}{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_file_name(format!("{stem}-x{ext}"))
}

/// Numeric param with fallback; blanks and junk fall back to the default.
fn param_num(params: &std::collections::HashMap<String, String>, key: &str, default: f64) -> f64 {
    params.get(key).and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

/// Lowercased string param with fallback.
fn param_str(params: &std::collections::HashMap<String, String>, key: &str, default: &str) -> String {
    params
        .get(key)
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Whitelisted extension param: a crafted `target` can never escape the
/// output folder or masquerade as another file type.
fn ext_in(params: &std::collections::HashMap<String, String>, key: &str, default: &str, allowed: &[&str]) -> String {
    let value = param_str(params, key, default);
    if allowed.contains(&value.as_str()) {
        value
    } else {
        default.to_string()
    }
}

fn param_file(params: &std::collections::HashMap<String, String>, key: &str, allowed_ext: &[&str], label: &str) -> Result<String, String> {
    let raw = params
        .get(key)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Choose the {label} first."))?;
    let path = std::path::PathBuf::from(&raw);
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| allowed_ext.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false);
    if !ext_ok || !path.is_file() {
        return Err(format!("That {label} file does not exist or is not supported."));
    }
    Ok(raw)
}

fn image_out_ext(params: &std::collections::HashMap<String, String>, input: &std::path::Path) -> String {
    let target = param_str(params, "target", "original");
    if ["jpg", "png", "webp"].contains(&target.as_str()) {
        return target;
    }
    input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .map(|e| if e == "jpeg" { "jpg".to_string() } else { e })
        .filter(|e| ["jpg", "png", "webp", "bmp", "gif"].contains(&e.as_str()))
        .unwrap_or_else(|| "png".to_string())
}

fn tool_args(request: &crate::models::ToolRequest, output: &std::path::Path) -> Result<Vec<String>, String> {
    let input = request.input.clone();
    let out = output.to_string_lossy().to_string();
    let p = &request.params;
    let mut args: Vec<String> = vec!["-y".to_string()];
    match request.tool.as_str() {
        "audio_mp3" => {
            args.extend(["-i".to_string(), input, "-vn".to_string(), "-codec:a".to_string(), "libmp3lame".to_string(), "-q:a".to_string(), "2".to_string()]);
            args.push(out);
            Ok(args)
        }
        "gif" => {
            let start = param_num(p, "start", 0.0).max(0.0);
            let duration = param_num(p, "duration", 5.0).clamp(0.2, 600.0);
            let fps = param_num(p, "fps", 10.0).clamp(1.0, 50.0).round() as u32;
            let width = param_num(p, "width", 480.0).clamp(64.0, 1920.0).round() as u32;
            args.extend(["-ss".to_string(), format!("{start}"), "-t".to_string(), format!("{duration}"), "-i".to_string(), input, "-vf".to_string(), format!("fps={fps},scale={width}:-1:flags=lanczos"), "-loop".to_string(), "0".to_string()]);
            args.push(out);
            Ok(args)
        }
        "compress" => {
            let crf = param_num(p, "crf", 30.0).clamp(18.0, 45.0).round() as u32;
            args.extend(["-i".to_string(), input, "-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), crf.to_string(), "-preset".to_string(), "medium".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "128k".to_string()]);
            args.push(out);
            Ok(args)
        }
        "trim" => {
            let start = param_num(p, "start", 0.0).max(0.0);
            let duration = param_num(p, "duration", 10.0).clamp(0.1, 24.0 * 3600.0);
            args.extend(["-ss".to_string(), format!("{start}"), "-t".to_string(), format!("{duration}"), "-i".to_string(), input, "-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "23".to_string(), "-preset".to_string(), "fast".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "128k".to_string(), "-movflags".to_string(), "+faststart".to_string()]);
            args.push(out);
            Ok(args)
        }
        "audio_convert" => {
            let format = ext_in(p, "format", "mp3", &["mp3", "flac", "ogg", "opus", "m4a", "wav"]);
            let bitrate = param_num(p, "bitrate", 192.0).clamp(32.0, 320.0).round() as u32;
            args.extend(["-i".to_string(), input, "-vn".to_string()]);
            match format.as_str() {
                "mp3" => args.extend(["-codec:a".to_string(), "libmp3lame".to_string(), "-b:a".to_string(), format!("{bitrate}k")]),
                "flac" => args.extend(["-codec:a".to_string(), "flac".to_string()]),
                "ogg" => args.extend(["-codec:a".to_string(), "libvorbis".to_string(), "-q:a".to_string(), "4".to_string()]),
                "opus" => args.extend(["-codec:a".to_string(), "libopus".to_string(), "-b:a".to_string(), format!("{bitrate}k")]),
                "m4a" => args.extend(["-codec:a".to_string(), "aac".to_string(), "-b:a".to_string(), format!("{bitrate}k")]),
                "wav" => args.extend(["-codec:a".to_string(), "pcm_s16le".to_string()]),
                _ => return Err("Unsupported audio format.".to_string()),
            }
            args.push(out);
            Ok(args)
        }
        "audio_lossless" => {
            // Matroska audio: every codec fits, copy always works.
            args.extend(["-i".to_string(), input, "-vn".to_string(), "-c:a".to_string(), "copy".to_string()]);
            args.push(out);
            Ok(args)
        }
        "audio_normalize" => {
            args.extend(["-i".to_string(), input, "-vn".to_string(), "-af".to_string(), "loudnorm=I=-16:TP=-1.5:LRA=11".to_string(), "-codec:a".to_string(), "libmp3lame".to_string(), "-q:a".to_string(), "2".to_string()]);
            args.push(out);
            Ok(args)
        }
        "audio_speed" => {
            let speed = param_num(p, "speed", 1.0).clamp(0.5, 4.0);
            let mut remaining = speed;
            let mut chain = String::new();
            while remaining > 2.0 {
                chain.push_str("atempo=2.0,");
                remaining /= 2.0;
            }
            chain.push_str(&format!("atempo={remaining}"));
            args.extend(["-i".to_string(), input, "-vn".to_string(), "-af".to_string(), chain, "-codec:a".to_string(), "libmp3lame".to_string(), "-q:a".to_string(), "2".to_string()]);
            args.push(out);
            Ok(args)
        }
        "video_convert" => {
            let target = ext_in(p, "target", "mp4", &["mp4", "mkv", "webm", "mov"]);
            let mode = param_str(p, "mode", "remux");
            if mode != "encode" && target != "webm" {
                // Stream copy: lossless, fast. WebM containers reject non-VP codecs.
                args.extend(["-i".to_string(), input, "-c".to_string(), "copy".to_string()]);
            } else if target == "webm" {
                args.extend(["-i".to_string(), input, "-c:v".to_string(), "libvpx-vp9".to_string(), "-crf".to_string(), "32".to_string(), "-b:v".to_string(), "0".to_string(), "-c:a".to_string(), "libopus".to_string(), "-b:a".to_string(), "128k".to_string()]);
            } else {
                args.extend(["-i".to_string(), input, "-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "23".to_string(), "-preset".to_string(), "fast".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "128k".to_string(), "-movflags".to_string(), "+faststart".to_string()]);
            }
            args.push(out);
            Ok(args)
        }
        "video_resize" => {
            let mut height = param_num(p, "height", 720.0).clamp(144.0, 4320.0).round() as u32;
            if height % 2 == 1 {
                height += 1;
            }
            args.extend(["-i".to_string(), input, "-vf".to_string(), format!("scale=-2:{height}"), "-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "23".to_string(), "-preset".to_string(), "fast".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "128k".to_string(), "-movflags".to_string(), "+faststart".to_string()]);
            args.push(out);
            Ok(args)
        }
        "video_rotate" => {
            let angle = param_num(p, "angle", 90.0) as u32;
            let vf = match angle {
                180 => "transpose=1,transpose=1",
                270 => "transpose=2",
                _ => "transpose=1",
            };
            args.extend(["-i".to_string(), input, "-vf".to_string(), vf.to_string(), "-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "23".to_string(), "-preset".to_string(), "fast".to_string(), "-c:a".to_string(), "copy".to_string()]);
            args.push(out);
            Ok(args)
        }
        "mute_video" => {
            args.extend(["-i".to_string(), input, "-c:v".to_string(), "copy".to_string(), "-an".to_string()]);
            args.push(out);
            Ok(args)
        }
        "extract_frame" => {
            let time = param_num(p, "time", 0.0).max(0.0);
            args.extend(["-ss".to_string(), format!("{time}"), "-i".to_string(), input, "-frames:v".to_string(), "1".to_string()]);
            args.push(out);
            Ok(args)
        }
        "subtitle_convert" => {
            args.extend(["-i".to_string(), input]);
            args.push(out);
            Ok(args)
        }
        "image_resize" => {
            let width = param_num(p, "width", 1280.0).clamp(16.0, 8192.0).round() as u32;
            args.extend(["-i".to_string(), input, "-vf".to_string(), format!("scale={width}:-1")]);
            args.push(out);
            Ok(args)
        }
        "image_convert" => {
            args.extend(["-i".to_string(), input]);
            args.push(out);
            Ok(args)
        }
        "waveform_png" => {
            args.extend(["-i".to_string(), input, "-filter_complex".to_string(), "aformat=channel_layouts=mono,showwavespic=s=1280x360:colors=0x4F8CFF".to_string(), "-frames:v".to_string(), "1".to_string()]);
            args.push(out);
            Ok(args)
        }
        "contact_sheet" => {
            // 20 frames spread across the duration, tiled 5 x 4.
            let fps = param_num(p, "fps", 1.0 / 3.0).max(0.01);
            args.extend(["-i".to_string(), input, "-vf".to_string(), format!("fps={fps},scale=320:-1,tile=5x4"), "-frames:v".to_string(), "1".to_string()]);
            args.push(out);
            Ok(args)
        }
        "volume_boost" => {
            let factor = param_num(p, "factor", 1.5).clamp(0.1, 10.0);
            args.extend(["-i".to_string(), input, "-vn".to_string(), "-af".to_string(), format!("volume={factor}"), "-codec:a".to_string(), "libmp3lame".to_string(), "-q:a".to_string(), "2".to_string()]);
            args.push(out);
            Ok(args)
        }
        "silence_trim" => {
            args.extend(["-i".to_string(), input, "-vn".to_string(), "-af".to_string(), "silenceremove=start_periods=1:start_threshold=-50dB:stop_periods=1:stop_threshold=-50dB".to_string(), "-codec:a".to_string(), "libmp3lame".to_string(), "-q:a".to_string(), "2".to_string()]);
            args.push(out);
            Ok(args)
        }
        "square_crop" => {
            args.extend(["-i".to_string(), input, "-vf".to_string(), "crop=ih:ih".to_string(), "-c:v".to_string(), "libx264".to_string(), "-crf".to_string(), "23".to_string(), "-preset".to_string(), "fast".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "128k".to_string(), "-movflags".to_string(), "+faststart".to_string()]);
            args.push(out);
            Ok(args)
        }
        "mux" => {
            let audio = param_file(p, "audio", &["mp3", "m4a", "wav", "flac", "ogg", "opus", "aac", "mka"], "audio")?;
            args.extend(["-i".to_string(), input, "-i".to_string(), audio, "-map".to_string(), "0:v?".to_string(), "-map".to_string(), "1:a?".to_string(), "-c:v".to_string(), "copy".to_string(), "-c:a".to_string(), "aac".to_string(), "-b:a".to_string(), "192k".to_string(), "-map_metadata".to_string(), "0".to_string(), "-movflags".to_string(), "+faststart".to_string()]);
            args.push(out);
            Ok(args)
        }
        "mp3_cover" => {
            let cover = param_file(p, "cover", &["jpg", "jpeg", "png", "webp"], "cover image")?;
            args.extend(["-i".to_string(), input, "-i".to_string(), cover, "-map".to_string(), "0:a".to_string(), "-map".to_string(), "1:0".to_string(), "-c:a".to_string(), "copy".to_string(), "-c:v".to_string(), "mjpeg".to_string(), "-id3v2_version".to_string(), "3".to_string(), "-metadata:s:v".to_string(), "title=Album cover".to_string(), "-metadata:s:v".to_string(), "comment=Cover (front)".to_string()]);
            args.push(out);
            Ok(args)
        }
        other => Err(format!("Unknown tool '{other}'.")),
    }
}

fn default_output(request: &crate::models::ToolRequest) -> std::path::PathBuf {
    let input = std::path::Path::new(&request.input);
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string();
    let parent = input.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let p = &request.params;
    match request.tool.as_str() {
        "audio_mp3" => parent.join(format!("{stem}.mp3")),
        "gif" => parent.join(format!("{stem}.gif")),
        "trim" => parent.join(format!("{stem}-trimmed.mp4")),
        "audio_convert" => parent.join(format!("{stem}.{}", ext_in(p, "format", "mp3", &["mp3", "flac", "ogg", "opus", "m4a", "wav"]))),
        "audio_lossless" => parent.join(format!("{stem}.mka")),
        "audio_normalize" => parent.join(format!("{stem}-normalized.mp3")),
        "audio_speed" => parent.join(format!("{stem}-speed.mp3")),
        "video_convert" => parent.join(format!("{stem}.{}", ext_in(p, "target", "mp4", &["mp4", "mkv", "webm", "mov"]))),
        "video_resize" => parent.join(format!("{stem}-{}p.mp4", param_num(p, "height", 720.0).clamp(144.0, 4320.0).round() as u32)),
        "video_rotate" => parent.join(format!("{stem}-rotated.mp4")),
        "mute_video" => parent.join(format!("{stem}-mute.mp4")),
        "extract_frame" => parent.join(format!("{stem}.{}", ext_in(p, "target", "jpg", &["jpg", "png", "webp"]))),
        "subtitle_convert" => parent.join(format!("{stem}.{}", ext_in(p, "target", "vtt", &["srt", "vtt", "ass"]))),
        "image_resize" => parent.join(format!("{stem}-resized.{}", image_out_ext(p, input))),
        "image_convert" => parent.join(format!("{stem}.{}", image_out_ext(p, input))),
        "waveform_png" => parent.join(format!("{stem}-waveform.png")),
        "contact_sheet" => parent.join(format!("{stem}-sheet.png")),
        "volume_boost" => parent.join(format!("{stem}-boosted.mp3")),
        "silence_trim" => parent.join(format!("{stem}-nosilence.mp3")),
        "square_crop" => parent.join(format!("{stem}-square.mp4")),
        "mux" => parent.join(format!("{stem}-merged.mp4")),
        "mp3_cover" => parent.join(format!("{stem}-cover.mp3")),
        _ => parent.join(format!("{stem}-compressed.mp4")),
    }
}

async fn probe_duration_secs(app_ffprobe: &std::path::Path, input: &std::path::Path) -> Option<f64> {
    let mut command = std::process::Command::new(app_ffprobe);
    command
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(input)
        .stdin(Stdio::null());
    ytdlp::hide_console_std(&mut command);
    let output = command.output().ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse::<f64>().ok().filter(|d| *d > 0.0)
}

pub async fn run_tool(app: &tauri::AppHandle, mut request: crate::models::ToolRequest) -> Result<String, String> {
    let input = std::path::PathBuf::from(&request.input);
    if !input.is_file() {
        return Err("That file no longer exists.".to_string());
    }
    if !matches!(
        request.tool.as_str(),
        "audio_mp3" | "gif" | "trim" | "compress"
            | "audio_convert" | "audio_lossless" | "audio_normalize" | "audio_speed"
            | "video_convert" | "video_resize" | "video_rotate" | "mute_video"
            | "extract_frame" | "subtitle_convert" | "image_resize" | "image_convert"
            | "waveform_png" | "volume_boost" | "silence_trim" | "square_crop" | "mux" | "mp3_cover" | "contact_sheet"
    ) {
        return Err(format!("Unknown tool '{}'.", request.tool));
    }
    let settings = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.lock().ok().map(|guard| guard.clone()))
        .unwrap_or_default();
    let data_dir = app
        .try_state::<AppState>()
        .map(|state| state.data_dir.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    let ffmpeg = ytdlp::locate_ffmpeg(settings.ffmpeg_path.as_deref(), &data_dir)
        .ok_or_else(|| "FFmpeg is not installed yet. Install it in Settings, Engine.".to_string())?;
    let ffprobe = ytdlp::locate_ffprobe(settings.ffmpeg_path.as_deref(), &data_dir);

    let output = match &request.output {
        Some(custom) if !custom.trim().is_empty() => std::path::PathBuf::from(custom),
        _ => unique_output(default_output(&request)),
    };
    if output == input {
        return Err("The output path must differ from the input file.".to_string());
    }

    let duration = match &ffprobe {
        Some(probe) => probe_duration_secs(probe, &input).await,
        None => None,
    };

    // Contact sheet: spread 20 frames evenly across the whole file.
    if request.tool.as_str() == "contact_sheet" {
        if let Some(d) = duration {
            if d > 0.0 {
                request.params.insert("fps".to_string(), (20.0 / d).to_string());
            }
        }
    }

    let mut args = tool_args(&request, &output)?;
    // Machine-readable progress goes to stdout; we parse it line by line.
    args.insert(0, "-nostats".to_string());
    args.insert(0, "-hide_banner".to_string());
    args.push("-progress".to_string());
    args.push("pipe:1".to_string());

    let mut ffmpeg_command = tokio::process::Command::new(&ffmpeg);
    ffmpeg_command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    ytdlp::hide_console_tokio(&mut ffmpeg_command);
    let mut child = ffmpeg_command
        .spawn()
        .map_err(|e| format!("Could not start FFmpeg: {e}"))?;

    let tool_name = request.tool.clone();
    if let Some(stdout) = child.stdout.take() {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(value) = line.trim().strip_prefix("out_time_us=") {
                if let Ok(us) = value.parse::<f64>() {
                    let percent = duration.map(|d| ((us / 1_000_000.0) / d * 100.0).clamp(0.0, 99.9));
                    if let Some(percent) = percent {
                        let _ = app.emit(
                            "tool-progress",
                            ToolProgressEvent { tool: tool_name.clone(), percent },
                        );
                    }
                }
            }
        }
    }
    // Drain stderr so the child can exit; keep the tail for the error report.
    let stderr = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        let mut tail: Vec<String> = Vec::new();
        if let Some(stream) = stderr {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tail.push(line);
                if tail.len() > 60 {
                    tail.remove(0);
                }
            }
        }
        tail
    });

    let status = child.wait().await.map_err(|e| format!("FFmpeg failed: {e}"))?;
    let stderr_tail = stderr_task.await.unwrap_or_default();

    let (success, message) = if status.success() {
        (true, None)
    } else {
        let message = stderr_tail
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| format!("FFmpeg exited with code {}", status.code().unwrap_or(-1)));
        (false, Some(message))
    };

    let _ = app.emit(
        "tool-finished",
        crate::models::ToolFinishedEvent {
            tool: tool_name,
            success,
            output: if success {
                Some(output.to_string_lossy().into_owned())
            } else {
                None
            },
            message,
        },
    );

    if success {
        Ok(output.to_string_lossy().into_owned())
    } else {
        Err(stderr_tail
            .iter()
            .rev()
            .find(|l| !l.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| "FFmpeg failed.".to_string()))
    }
}

/* ========== analysis (preview without downloading) ========== */

pub async fn analyze(app: &tauri::AppHandle, url: &str) -> Result<Analysis, String> {
    let trimmed = validate_url(url)?;
    let is_search = trimmed.to_ascii_lowercase().starts_with("ytsearch");
    let platform_name = if is_search {
        "YouTube Search".to_string()
    } else {
        platform::detect(&trimmed).display_name
    };
    if !is_search {
        let platform_info = platform::detect(&trimmed);
        if !platform_info.supported {
            return Ok(Analysis {
                platform: platform_info.display_name,
                supported: false,
                title: None,
                author: None,
                thumbnail_url: None,
                duration_seconds: None,
                media_kind: None,
                is_playlist: false,
                entry_count: None,
                qualities: Vec::new(),
                playlist_entries: Vec::new(),
            });
        }
    }

    let settings = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.lock().ok().map(|guard| guard.clone()))
        .unwrap_or_default();
    let data_dir = app
        .try_state::<AppState>()
        .map(|state| state.data_dir.clone())
        .unwrap_or_else(|| PathBuf::from("."));
    let ytdlp_path = ytdlp::locate_ytdlp(settings.ytdlp_path.as_deref(), &data_dir)
        .ok_or_else(|| "The download engine is not installed. Install it in Settings, Engine.".to_string())?;

    let mut command = tokio::process::Command::new(&ytdlp_path);
    command
        .args([
            "--no-colors",
            "--flat-playlist",
            "--dump-single-json",
            "--no-warnings",
        ])
        .args(metadata_args(&settings))
        .arg(trimmed)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PYTHONUTF8", "1")
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .kill_on_drop(true);
    ytdlp::hide_console_tokio(&mut command);

    let output = tokio::time::timeout(Duration::from_secs(45), command.output())
        .await
        .map_err(|_| "Timed out while reading this link.".to_string())?
        .map_err(|e| format!("Could not run yt-dlp: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail = stderr
            .lines()
            .rev()
            .find(|l| l.trim().starts_with("ERROR:"))
            .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()))
            .unwrap_or("yt-dlp failed");
        return Err(tail.trim().to_string());
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Could not parse the engine response: {e}"))?;

    Ok(analysis_from_json(&value, &platform_name))
}

fn analysis_from_json(value: &serde_json::Value, platform_name: &str) -> Analysis {
    let entries = value.get("entries").and_then(|e| e.as_array());
    let is_playlist = entries.is_some();
    let entry_count = entries.map(|e| e.len() as u32);

    fn clip_title(text: &str) -> String {
        let mut t: String = text.chars().take(180).collect();
        if text.chars().count() > 180 {
            t.push('…');
        }
        t
    }

    let playlist_entries: Vec<crate::models::PlaylistEntry> = entries
        .map(|e| {
            e.iter()
                .take(200)
                .enumerate()
                .map(|(i, entry)| crate::models::PlaylistEntry {
                    index: (i + 1) as u32,
                    title: entry
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(clip_title)
                        .unwrap_or_else(|| "(untitled)".to_string()),
                    url: entry
                        .get("url")
                        .and_then(|v| v.as_str())
                        .or_else(|| entry.get("webpage_url").and_then(|v| v.as_str()))
                        .map(String::from)
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();

    // For playlists, preview the first entry rather than nothing.
    let focus: &serde_json::Value = if is_playlist {
        value.get("entries").and_then(|e| e.get(0)).unwrap_or(value)
    } else {
        value
    };

    let title = value.get("title").and_then(|v| v.as_str()).map(String::from)
        .or_else(|| focus.get("title").and_then(|v| v.as_str()).map(String::from));
    let author = value.get("uploader").and_then(|v| v.as_str()).map(String::from)
        .or_else(|| value.get("channel").and_then(|v| v.as_str()).map(String::from))
        .or_else(|| focus.get("uploader").and_then(|v| v.as_str()).map(String::from));
    let thumbnail_url = value.get("thumbnail").and_then(|v| v.as_str()).map(String::from)
        .or_else(|| focus.get("thumbnail").and_then(|v| v.as_str()).map(String::from));
    let duration_seconds = value.get("duration").and_then(|v| v.as_f64())
        .or_else(|| focus.get("duration").and_then(|v| v.as_f64()));
    let media_kind = value.get("_type").and_then(|v| v.as_str()).map(String::from);

    // Collect unique resolutions from the format list for the quality picker.
    let mut heights: HashMap<u32, (u32, Option<String>)> = HashMap::new();
    if let Some(formats) = value.get("formats").and_then(|f| f.as_array()) {
        for format in formats {
            let height = format.get("height").and_then(|v| v.as_u64()).map(|h| h as u32);
            let width = format.get("width").and_then(|v| v.as_u64()).map(|w| w as u32);
            let ext = format.get("ext").and_then(|v| v.as_str()).map(String::from);
            if let Some(h) = height {
                if h >= 144 {
                    heights.entry(h).or_insert((width.unwrap_or(0), ext));
                }
            }
        }
    }
    let mut qualities: Vec<crate::models::VideoQuality> = heights
        .into_iter()
        .map(|(h, (w, ext))| crate::models::VideoQuality {
            label: format!("{h}p"),
            width: Some(w),
            height: Some(h),
            format: ext,
        })
        .collect();
    qualities.sort_by_key(|q| std::cmp::Reverse(q.height.unwrap_or(0)));

    Analysis {
        platform: platform_name.to_string(),
        supported: true,
        title,
        author,
        thumbnail_url,
        duration_seconds,
        media_kind,
        is_playlist,
        entry_count,
        qualities,
        playlist_entries,
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings() -> crate::settings::AppSettings {
        crate::settings::AppSettings::default()
    }

    #[test]
    fn build_args_audio_mode_extracts() {
        let args = build_args("https://example.com/v", DownloadMode::Audio, "best", None, None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("-x"));
        assert!(joined.contains("--audio-format m4a"));
        assert!(joined.contains("/tmp/out/"));
    }

    #[test]
    fn build_args_forces_progress_output() {
        let args = build_args("https://example.com/v", DownloadMode::Auto, "best", None, None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("--progress"), "progress must be enabled even with --print");
        assert!(joined.contains("--newline"));
        assert!(joined.contains("after_move:GettaFile:%(filepath)s"));
    }

    #[test]
    fn build_args_download_archive_when_enabled() {
        let mut settings = test_settings();
        settings.download_archive = true;
        let args = build_args("https://example.com/v", DownloadMode::Auto, "best", None, None, &settings, "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("--download-archive"));
        assert!(joined.contains("download-archive.txt"));
    }

    #[test]
    fn build_args_metadata_args_shared() {
        let mut settings = test_settings();
        settings.proxy = "http://127.0.0.1:8080".to_string();
        let parsed = metadata_args(&settings);
        assert_eq!(parsed, vec!["--proxy".to_string(), "http://127.0.0.1:8080".to_string()]);
    }

    #[test]
    fn build_args_quality_cap() {
        let args = build_args("https://example.com/v", DownloadMode::Auto, "1080", None, None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b"));
    }

    #[test]
    fn build_args_mute_video_only() {
        let args = build_args("https://example.com/v", DownloadMode::Mute, "720", None, None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("bv*[height<=720]/bv*"));
        assert!(!joined.contains("+ba"));
    }

    #[test]
    fn build_args_respects_speed_and_cookies() {
        let mut settings = test_settings();
        settings.speed_limit_mbps = 5;
        settings.cookies_from_browser = "firefox".to_string();
        let args = build_args("https://example.com/v", DownloadMode::Auto, "best", None, None, &settings, "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("--limit-rate 5M"));
        assert!(joined.contains("--cookies-from-browser firefox"));
    }

    #[test]
    fn build_args_organize_by_platform_prefixes_template() {
        let mut settings = test_settings();
        settings.organize_by_platform = true;
        let args = build_args("https://example.com/v", DownloadMode::Auto, "best", None, None, &settings, "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("/tmp/out/%(extractor_key)s/"));
    }

    #[test]
    fn validate_url_accepts_http_only() {
        assert_eq!(validate_url("https://example.com/watch?v=1").unwrap(), "https://example.com/watch?v=1");
        assert_eq!(validate_url("  https://example.com/  ").unwrap(), "https://example.com/");
        assert!(validate_url("ftp://example.com/file").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
        assert!(validate_url("magnet:?xt=urn:btih:abc").is_err());
        assert_eq!(validate_url("https://example.com/file.torrent").unwrap_err(), "Torrents and magnet links arrive in a later release.");
    }
    #[test]
    fn title_from_url_falls_back_to_host() {
        assert_eq!(title_from_url("https://example.com/path/to/file.mp4"), "file.mp4");
        assert_eq!(title_from_url("https://example.com/"), "example.com");
    }
}
