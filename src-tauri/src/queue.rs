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

use crate::models::{Analysis, DownloadMode, QueueItemInfo, QueueStatus};
use crate::platform;
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
        let trimmed = url.trim();
        if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
            if trimmed.starts_with("magnet:") || trimmed.to_ascii_lowercase().ends_with(".torrent") {
                return Err("Torrents and magnet links arrive in a later release.".to_string());
            }
            return Err("Only http(s) links are supported.".to_string());
        }
        let platform_info = platform::detect(trimmed);
        if !platform_info.supported {
            return Err(format!("{} links are not supported yet.", platform_info.display_name));
        }

        let item = QueueItem {
            id: next_id(),
            url: trimmed.to_string(),
            platform: platform_info.display_name.clone(),
            title: title_from_url(trimmed),
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
        };
        let id = item.id;
        {
            self.inner.lock().expect("queue mutex").items.push(item);
        }
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
            queued.sort_unstable();
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
        let pid = {
            let mut inner = self.inner.lock().expect("queue mutex");
            let item = inner.items.iter_mut().find(|i| i.id == id).ok_or("item not found")?;
            if item.status != QueueStatus::Paused {
                return Err("Only paused downloads can be resumed.".to_string());
            }
            item.pid
        };
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
        emit_queue(app);
        self.try_start_next(app);
        sync_prevent_sleep(app);
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
    let _ = app.emit("queue-changed", snapshot);
}

fn title_from_url(url: &str) -> String {
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

fn build_args(item_url: &str, mode: DownloadMode, quality: &str, format_id: Option<&str>, settings: &crate::settings::AppSettings, output_dir: &str) -> Vec<String> {
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
    if settings.speed_limit_mbps > 0 {
        args.push("--limit-rate".to_string());
        args.push(format!("{}M", settings.speed_limit_mbps));
    }
    if settings.concurrent_fragments > 0 {
        args.push("--concurrent-fragments".to_string());
        args.push(settings.concurrent_fragments.to_string());
    }
    if !settings.cookies_from_browser.trim().is_empty() {
        args.push("--cookies-from-browser".to_string());
        args.push(settings.cookies_from_browser.trim().to_string());
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
        let args = build_args(&item.url, item.download_mode, &item.quality, item.format_id.as_deref(), &settings, &output_dir);
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

/* ========== analysis (preview without downloading) ========== */

pub async fn analyze(app: &tauri::AppHandle, url: &str) -> Result<Analysis, String> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("Only http(s) links are supported right now.".to_string());
    }
    let platform_info = platform::detect(trimmed);
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
        });
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

    Ok(analysis_from_json(&value, &platform_info.display_name))
}

fn analysis_from_json(value: &serde_json::Value, platform_name: &str) -> Analysis {
    let entries = value.get("entries").and_then(|e| e.as_array());
    let is_playlist = entries.is_some();
    let entry_count = entries.map(|e| e.len() as u32);

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
        let args = build_args("https://example.com/v", DownloadMode::Audio, "best", None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("-x"));
        assert!(joined.contains("--audio-format m4a"));
        assert!(joined.contains("/tmp/out/"));
    }

    #[test]
    fn build_args_quality_cap() {
        let args = build_args("https://example.com/v", DownloadMode::Auto, "1080", None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b"));
    }

    #[test]
    fn build_args_mute_video_only() {
        let args = build_args("https://example.com/v", DownloadMode::Mute, "720", None, &test_settings(), "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("bv*[height<=720]/bv*"));
        assert!(!joined.contains("+ba"));
    }

    #[test]
    fn build_args_respects_speed_and_cookies() {
        let mut settings = test_settings();
        settings.speed_limit_mbps = 5;
        settings.cookies_from_browser = "firefox".to_string();
        let args = build_args("https://example.com/v", DownloadMode::Auto, "best", None, &settings, "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("--limit-rate 5M"));
        assert!(joined.contains("--cookies-from-browser firefox"));
    }

    #[test]
    fn build_args_organize_by_platform_prefixes_template() {
        let mut settings = test_settings();
        settings.organize_by_platform = true;
        let args = build_args("https://example.com/v", DownloadMode::Auto, "best", None, &settings, "/tmp/out");
        let joined = args.join(" ");
        assert!(joined.contains("/tmp/out/%(extractor_key)s/"));
    }

    #[test]
    fn title_from_url_falls_back_to_host() {
        assert_eq!(title_from_url("https://example.com/path/to/file.mp4"), "file.mp4");
        assert_eq!(title_from_url("https://example.com/"), "example.com");
    }
}
