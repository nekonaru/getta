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

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::models::{Analysis, DownloadMode, QueueItemInfo, QueueStatus, RecoveryEntry};
use crate::platform;
use crate::recovery;
use crate::ytdlp;
use crate::{AppState, history};

use tauri::{Emitter, Manager};

static ID_SEED: AtomicU64 = AtomicU64::new(0);

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
    started_at: Option<i64>,
    output_dir: Option<String>,
    playlist_items: Option<String>,
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
        self.inner.lock().expect("queue mutex").items.iter().map(QueueItem::info).collect()
    }

    pub fn start(&self, app: &tauri::AppHandle, url: &str, options: &crate::models::StartOptions) -> Result<u64, String> {
        let trimmed = validate_url(url)?;
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
            download_mode: DownloadMode::parse(&options.download_mode),
            quality: if options.quality.trim().is_empty() { "best".to_string() } else { options.quality.trim().to_string() },
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
            started_at: None,
            output_dir: options.output_dir.clone().filter(|d| !d.trim().is_empty()),
            playlist_items: options
                .playlist_items
                .clone()
                .filter(|s| !s.trim().is_empty())
                .filter(|s| s.chars().all(|c| c.is_ascii_digit() || matches!(c, ',' | '-' | ' '))),
        };
        let item_title = item.title.clone();
        let id = item.id;
        {
            self.inner.lock().expect("queue mutex").items.push(item);
        }
        // Remember it: if the process dies mid-download, launch can offer it back.
        let recovery_entry = RecoveryEntry {
            id,
            url: trimmed.to_string(),
            title: item_title,
            download_mode: options.download_mode.clone(),
            quality: options.quality.clone(),
            format_id: options.format_id.clone(),
            playlist_items: options.playlist_items.clone(),
            output_dir: options.output_dir.clone(),
        };
        let _ = recovery::push(&self.data_dir, recovery_entry);
        emit_queue(app);
        self.try_start_next(app);
        Ok(id)
    }

    pub fn try_start_next(&self, app: &tauri::AppHandle) {
        let max_concurrent = app
            .try_state::<AppState>()
            .and_then(|state| state.settings.lock().ok().map(|s| s.max_concurrent))
            .unwrap_or(2)
            .max(1);
        let mut to_spawn = Vec::new();
        {
            let mut inner = self.inner.lock().expect("queue mutex");
            let running = inner.items.iter().filter(|i| i.status == QueueStatus::Active).count() as u32;
            let free = max_concurrent.saturating_sub(running);
            let mut queued: Vec<u64> = inner
                .items
                .iter()
                .filter(|i| i.status == QueueStatus::Queued)
                .map(|i| i.id)
                .collect();
            // Keep the user's manual order (drag / move up/down): spawn in Vec order.
            for id in queued.into_iter().take(free as usize) {
                if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                    item.status = QueueStatus::Active;
                    item.phase = Some("starting".to_string());
                    item.started_at = Some(now_secs());
                    to_spawn.push(id);
                }
            }
        }
        for id in to_spawn {
            spawn_job(app.clone(), id);
        }
        sync_prevent_sleep(app);
    }

    pub fn pause(&self, app: &tauri::AppHandle, id: u64) -> Result<(), String> {
        let pid = {
            let inner = self.inner.lock().expect("queue mutex");
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
            let mut inner = self.inner.lock().expect("queue mutex");
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
            let inner = self.inner.lock().expect("queue mutex");
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
                let mut inner = self.inner.lock().expect("queue mutex");
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
                let mut inner = self.inner.lock().expect("queue mutex");
                if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                    // yt-dlp picks up from the .part file in the output dir.
                    item.pid = None;
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
        let pid = {
            let mut inner = self.inner.lock().expect("queue mutex");
            let Some(index) = inner.items.iter().position(|i| i.id == id) else {
                return;
            };
            inner.items[index].cancelled = true;
            inner.items.remove(index).pid
        };
        if let Some(pid) = pid {
            kill_process(pid);
        }
        let _ = recovery::remove(&self.data_dir, id);
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
    }

    pub fn retry(&self, app: &tauri::AppHandle, id: u64) {
        {
            let mut inner = self.inner.lock().expect("queue mutex");
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                if item.status != QueueStatus::Error {
                    return;
                }
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
        let (pid, file_path) = {
            let mut inner = self.inner.lock().expect("queue mutex");
            let Some(index) = inner.items.iter().position(|i| i.id == id) else {
                return;
            };
            let item = inner.items.remove(index);
            (item.pid, if delete_file { item.file_path } else { None })
        };
        if let Some(pid) = pid {
            kill_process(pid);
        }
        if let Some(path) = file_path {
            let _ = std::fs::remove_file(&path);
        }
        let _ = recovery::remove(&self.data_dir, id);
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
    }

    pub fn move_item(&self, app: &tauri::AppHandle, id: u64, delta: i32) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().expect("queue mutex");
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

    pub fn clear_finished(&self, app: &tauri::AppHandle) {
        {
            let mut inner = self.inner.lock().expect("queue mutex");
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
    let ids: Vec<u64> = app
        .try_state::<AppState>()
        .map(|state| {
            state
                .queue
                .snapshot()
                .into_iter()
                .filter(|i| i.status == "active")
                .map(|i| i.id)
                .collect()
        })
        .unwrap_or_default();
    if let Some(state) = app.try_state::<AppState>() {
        for id in ids {
            let _ = state.queue.pause(app, id);
        }
    }
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

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
        "--no-colors".to_string(),
        "--print".to_string(),
        "after_move:filepath".to_string(),
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

    match mode {
        DownloadMode::Audio => {
            args.push("-x".to_string());
            args.push("--audio-format".to_string());
            args.push("m4a".to_string());
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
    {
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
    }
    if !settings.proxy.trim().is_empty() {
        args.push("--proxy".to_string());
        args.push(settings.proxy.trim().to_string());
    }

    args.push("--yes-playlist".to_string());
    args.push(item_url.to_string());
    args
}

fn spawn_job(app: tauri::AppHandle, id: u64) {
    tauri::async_runtime::spawn(async move {
        run_job(app, id).await;
    });
}

async fn run_job(app: tauri::AppHandle, id: u64) {
    let (output_dir, args, ytdlp_path) = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let settings = state.settings.lock().expect("settings mutex").clone();
        let Some(item) = state.queue.inner.lock().expect("queue mutex").items.iter().find(|i| i.id == id).cloned() else {
            return;
        };
        let output_dir = item
            .output_dir
            .clone()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| settings.output_dir.clone());
        let ytdlp_path = ytdlp::locate_ytdlp(settings.ytdlp_path.as_deref(), &state.data_dir);
        let args = build_args(&item.url, item.download_mode, &item.quality, item.format_id.as_deref(), item.playlist_items.as_deref(), &settings, &output_dir);
        (output_dir, args, ytdlp_path)
    };

    if std::fs::create_dir_all(&output_dir).is_err() {
        finalize_error(&app, id, "The download folder cannot be created.", false);
        return;
    }
    let Some(ytdlp_path) = ytdlp_path else {
        finalize_error(&app, id, "The download engine is not installed. Install it in Settings, Engine.", false);
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
            finalize_error(&app, id, &format!("Could not start yt-dlp: {e}"), false);
            return;
        }
    };

    {
        if let Some(state) = app.try_state::<AppState>() {
            let mut inner = state.queue.inner.lock().expect("queue mutex");
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
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
                ytdlp::LineEvent::Progress { percent, total_bytes, speed_bps, eta_seconds, .. } => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().expect("queue mutex");
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            if item.cancelled {
                                continue;
                            }
                            item.percent = percent;
                            item.total_bytes = total_bytes.or(item.total_bytes);
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
                ytdlp::LineEvent::Destination(path) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().expect("queue mutex");
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            item.file_path = Some(path);
                            item.file_count = Some(item.file_count.unwrap_or(0) + 1);
                            item.phase = Some("saving".to_string());
                        }
                    }
                    structural_emit = true;
                }
                ytdlp::LineEvent::PostProcess(phase) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().expect("queue mutex");
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            item.phase = Some(phase);
                        }
                    }
                    structural_emit = true;
                }
                ytdlp::LineEvent::PlaylistItem(index) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let mut inner = state.queue.inner.lock().expect("queue mutex");
                        if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
                            item.phase = Some(format!("item {index}"));
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
    let cancelled = app
        .try_state::<AppState>()
        .map(|state| state.queue.inner.lock().map(|inner| inner.items.iter().all(|i| i.id != id)).unwrap_or(true))
        .unwrap_or(true);

    if cancelled {
        // The item was cancelled or removed while running; nothing left to do.
        emit_queue(&app);
        return;
    }

    match exit {
        Ok(status) if status.success() => {
            let (title, url, file_path, file_size, platform, thumbnail_url, quality, mode) = {
                let Some(state) = app.try_state::<AppState>() else { return };
                let mut inner = state.queue.inner.lock().expect("queue mutex");
                let Some(item) = inner.items.iter_mut().find(|i| i.id == id) else { return };
                item.status = QueueStatus::Done;
                item.percent = 100.0;
                item.speed_bps = 0;
                item.eta_seconds = None;
                item.phase = None;
                let size = item.file_path.as_ref().and_then(|p| std::fs::metadata(p).ok()).map(|m| m.len());
                item.downloaded_bytes = item.total_bytes.or(Some(item.downloaded_bytes)).unwrap_or(item.downloaded_bytes);
                (
                    item.title.clone(),
                    item.url.clone(),
                    item.file_path.clone(),
                    size,
                    item.platform.clone(),
                    item.thumbnail_url.clone(),
                    item.quality.clone(),
                    item.download_mode.as_str().to_string(),
                )
            };
            if let Some(state) = app.try_state::<AppState>() {
                history::push(
                    &state.data_dir,
                    crate::models::HistoryEntry {
                        id: uuid::Uuid::new_v4().to_string(),
                        url,
                        platform,
                        title,
                        file_path,
                        file_size_bytes: file_size,
                        success: true,
                        error: None,
                        completed_at: now_secs(),
                        thumbnail_url,
                        quality: Some(quality),
                        download_mode: Some(mode),
                    },
                );
                let _ = recovery::remove(&state.data_dir, id);
            }
            emit_queue(&app);
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
            finalize_maybe_retry(&app, id, &message);
        }
        Err(e) => {
            finalize_error(&app, id, &format!("yt-dlp failed to run: {e}"), false);
        }
    }
}

fn finalize_maybe_retry(app: &tauri::AppHandle, id: u64, message: &str) {
    let max_retries = app
        .try_state::<AppState>()
        .and_then(|state| state.settings.lock().ok().map(|s| s.max_retries))
        .unwrap_or(3);

    let (category, retryable) = ytdlp::classify_error(message);
    let (retry_count, big_enough) = {
        let Some(state) = app.try_state::<AppState>() else { return };
        let inner = state.queue.inner.lock().expect("queue mutex");
        match inner.items.iter().find(|i| i.id == id) {
            Some(item) => (item.retry_count, item.downloaded_bytes > 5 * 1024 * 1024),
            None => return,
        }
    };

    let can_retry = retryable && (category == "rate_limited" || category == "unknown")
        && retry_count < max_retries;

    if can_retry {
        let delay_secs = 2u64.pow(retry_count.saturating_add(1).min(6)).min(30);
        if let Some(state) = app.try_state::<AppState>() {
            let mut inner = state.queue.inner.lock().expect("queue mutex");
            if let Some(item) = inner.items.iter_mut().find(|i| i.id == id) {
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
        let mut inner = state.queue.inner.lock().expect("queue mutex");
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

pub async fn run_tool(app: &tauri::AppHandle, request: crate::models::ToolRequest) -> Result<String, String> {
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
        let tail = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("yt-dlp failed");
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
