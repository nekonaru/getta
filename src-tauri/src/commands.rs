//! IPC commands. Every command returns `Result<T, String>`; errors are
//! already sanitized for display.

use std::process::Command;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::history;
use crate::models::{Analysis, DependencyInfo, EngineStatus, StartOptions};
use crate::settings::AppSettings;
use crate::ytdlp;
use crate::AppState;

#[tauri::command]
pub async fn analyze_url(app: AppHandle, url: String) -> Result<Analysis, String> {
    crate::queue::analyze(&app, &url).await
}

#[tauri::command]
pub async fn run_tool(
    app: AppHandle,
    request: crate::models::ToolRequest,
) -> Result<String, String> {
    crate::queue::run_tool(&app, request).await
}

#[tauri::command]
pub async fn start_download(app: AppHandle, options: StartOptions) -> Result<u64, String> {
    app.try_state::<AppState>()
        .ok_or_else(|| "App is not ready yet.".to_string())?
        .queue
        .start(&app, &options.url, &options)
}

#[tauri::command]
pub async fn pause_download(app: AppHandle, id: u64) -> Result<(), String> {
    app.try_state::<AppState>()
        .ok_or_else(|| "App is not ready yet.".to_string())?
        .queue
        .pause(&app, id)
}

#[tauri::command]
pub async fn resume_download(app: AppHandle, id: u64) -> Result<(), String> {
    app.try_state::<AppState>()
        .ok_or_else(|| "App is not ready yet.".to_string())?
        .queue
        .resume(&app, id)
}

#[tauri::command]
pub async fn cancel_download(app: AppHandle, id: u64) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.cancel(&app, id);
    }
    Ok(())
}

#[tauri::command]
pub async fn retry_download(app: AppHandle, id: u64) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.retry(&app, id);
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_download(app: AppHandle, id: u64, delete_file: bool) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.remove(&app, id, delete_file);
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_finished(app: AppHandle) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.clear_finished(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn pause_all(app: AppHandle) -> Result<(), String> {
    crate::queue::pause_all(&app)
}

#[tauri::command]
pub async fn resume_all(app: AppHandle) -> Result<(), String> {
    crate::queue::resume_all(&app)
}

#[tauri::command]
pub async fn move_item(state: State<'_, AppState>, app: AppHandle, id: u64, delta: i32) -> Result<(), String> {
    state.queue.move_item(&app, id, delta)
}

#[tauri::command]
pub async fn get_recovery_items(state: State<'_, AppState>) -> Result<Vec<crate::models::RecoveryEntry>, String> {
    Ok(crate::recovery::list(&state.data_dir))
}

#[tauri::command]
pub async fn restore_recovery(app: AppHandle, state: State<'_, AppState>) -> Result<u64, String> {
    let entries = crate::recovery::list(&state.data_dir);
    let mut restored = 0u64;
    for entry in entries {
        let options = StartOptions {
            url: entry.url,
            download_mode: entry.download_mode,
            quality: entry.quality,
            format_id: entry.format_id,
            playlist_items: entry.playlist_items,
            output_dir: entry.output_dir,
        };
        if state.queue.start(&app, &options.url, &options).is_ok() {
            restored += 1;
        }
    }
    crate::recovery::clear(&state.data_dir).map_err(|e| e.to_string())?;
    Ok(restored)
}

#[tauri::command]
pub async fn dismiss_recovery(state: State<'_, AppState>) -> Result<(), String> {
    crate::recovery::clear(&state.data_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn queue_snapshot(state: State<'_, AppState>, app: AppHandle) -> Result<Vec<crate::models::QueueItemInfo>, String> {
    // Heal the queue: if an item is queued but capacity is free, spawn it.
    // Idempotent and safe to call on every snapshot.
    state.queue.try_start_next(&app);
    Ok(state.queue.snapshot())
}

#[tauri::command]
pub async fn history_list(state: State<'_, AppState>) -> Result<Vec<crate::models::HistoryEntry>, String> {
    Ok(history::list(&state.data_dir))
}

#[tauri::command]
pub async fn follow_list(state: State<'_, AppState>) -> Result<Vec<crate::models::FollowChannel>, String> {
    Ok(crate::follows::list(&state.data_dir))
}

#[tauri::command]
pub async fn follow_add(app: AppHandle, url: String) -> Result<crate::models::FollowChannel, String> {
    crate::follows::follow_add(&app, &url).await
}

#[tauri::command]
pub async fn follow_remove(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    crate::follows::remove(&state.data_dir, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn follow_set_auto(state: State<'_, AppState>, id: String, auto: bool) -> Result<bool, String> {
    crate::follows::set_auto(&state.data_dir, &id, auto).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn follow_check_now(app: AppHandle, id: String) -> Result<usize, String> {
    crate::follows::check_and_enqueue(&app, &id).await
}

#[tauri::command]
pub async fn get_clipboard_text(app: AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|e| format!("clipboard unreadable: {e}"))
}

#[tauri::command]
pub async fn set_clipboard_text(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|e| format!("clipboard unreadable: {e}"))
}

#[tauri::command]
pub async fn export_history_csv(state: State<'_, AppState>, path: String) -> Result<(), String> {
    history::export_csv(&state.data_dir, std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_history_json(state: State<'_, AppState>, path: String) -> Result<(), String> {
    history::export_json(&state.data_dir, std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cookie_bucket_check(path: String) -> Result<crate::cookies::CookieReport, String> {
    Ok(crate::cookies::check_path(&path))
}

#[tauri::command]
pub async fn probe_chapters(state: State<'_, AppState>, path: String) -> Result<Vec<crate::models::ChapterMarker>, String> {
    let input = std::path::PathBuf::from(&path);
    if !input.is_file() {
        return Err("That file no longer exists.".to_string());
    }
    let settings = state.settings.lock().map_err(|_| "settings locked".to_string())?.clone();
    let Some(ffprobe) = ytdlp::locate_ffprobe(settings.ffmpeg_path.as_deref(), &state.data_dir) else {
        return Err("ffprobe is not installed. Install FFmpeg in Settings, Engine.".to_string());
    };
    let mut command = std::process::Command::new(&ffprobe);
    command
        .args(["-v", "error", "-show_chapters", "-of", "json"])
        .arg(&input)
        .stdin(std::process::Stdio::null());
    ytdlp::hide_console_std(&mut command);
    let output = command.output().map_err(|e| format!("Could not run ffprobe: {e}"))?;
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Could not parse the ffprobe response: {e}"))?;
    let chapters = value.get("chapters").and_then(|c| c.as_array()).cloned().unwrap_or_default();
    let mut markers = Vec::with_capacity(chapters.len());
    for chapter in chapters {
        let read_time = |key: &str| -> f64 {
            chapter
                .get(key)
                .and_then(|v| {
                    v.as_str()
                        .and_then(|s| s.parse::<f64>().ok())
                        .or_else(|| v.as_f64())
                })
                .unwrap_or(0.0)
        };
        let title = chapter
            .get("tags")
            .and_then(|t| t.get("title"))
            .and_then(|t| t.as_str())
            .map(String::from);
        let start = read_time("start_time");
        let end = read_time("end_time").max(start);
        markers.push(crate::models::ChapterMarker {
            title,
            start_seconds: start,
            end_seconds: end,
        });
    }
    Ok(markers)
}

#[tauri::command]
pub async fn find_duplicates(state: State<'_, AppState>) -> Result<Vec<Vec<String>>, String> {
    fn sha256_file(path: &std::path::Path) -> Option<String> {
        use sha2::Digest;
        let mut file = std::fs::File::open(path).ok()?;
        let mut hasher = sha2::Sha256::new();
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            match std::io::Read::read(&mut file, &mut buffer) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buffer[..n]),
                Err(_) => return None,
            }
        }
        Some(hex::encode(hasher.finalize()))
    }

    let mut by_size: std::collections::HashMap<u64, Vec<(String, std::path::PathBuf)>> =
        std::collections::HashMap::new();
    for entry in history::list(&state.data_dir) {
        if !entry.success {
            continue;
        }
        let Some(path) = entry.file_path else { continue };
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        if meta.len() == 0 {
            continue;
        }
        by_size.entry(meta.len()).or_default().push((entry.id, std::path::PathBuf::from(path)));
    }

    let mut by_hash: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (_, candidates) in by_size {
        if candidates.len() < 2 {
            continue;
        }
        for (id, path) in candidates {
            if let Some(hash) = sha256_file(&path) {
                by_hash.entry(hash).or_default().push(id);
            }
        }
    }

    Ok(by_hash.into_values().filter(|ids| ids.len() >= 2).collect())
}

#[tauri::command]
pub async fn history_clear(state: State<'_, AppState>) -> Result<(), String> {
    history::clear(&state.data_dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn history_remove(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    history::remove(&state.data_dir, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().map_err(|_| "settings locked".to_string())?.clone())
}

#[tauri::command]
pub async fn settings_update(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: serde_json::Value,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().map_err(|_| "settings locked".to_string())?;
    // Merge over a JSON value of the current settings so old clients and
    // future fields degrade gracefully.
    let mut current = serde_json::to_value(&*settings).map_err(|e| e.to_string())?;
    if let (Some(target), Some(source)) = (current.as_object_mut(), patch.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    let merged: AppSettings = serde_json::from_value(current).map_err(|e| e.to_string())?;
    crate::settings::save(&state.data_dir, &merged).map_err(|e| e.to_string())?;
    *settings = merged.clone();
    drop(settings);
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.try_start_next(&app);
    }
    Ok(merged)
}

#[tauri::command]
pub async fn settings_reset(app: AppHandle, state: State<'_, AppState>) -> Result<AppSettings, String> {
    let fresh = crate::settings::reset(&state.data_dir).map_err(|e| e.to_string())?;
    {
        let mut settings = state.settings.lock().map_err(|_| "settings locked".to_string())?;
        *settings = fresh.clone();
    }
    state.queue.try_start_next(&app);
    Ok(fresh)
}

fn dependency_info(
    name: &str,
    found: Option<std::path::PathBuf>,
    managed: std::path::PathBuf,
    override_path: Option<&str>,
) -> DependencyInfo {
    match found {
        Some(path) => {
            let version = if name == "yt-dlp" {
                ytdlp::version_of(&path, &["--version"])
            } else {
                ytdlp::version_of(&path, &["-version"])
            };
            let is_custom = override_path.map(str::trim).filter(|s| !s.is_empty()).is_some();
            let source = if is_custom {
                "custom"
            } else if path == managed {
                "managed"
            } else {
                "system"
            };
            DependencyInfo {
                name: name.to_string(),
                found: true,
                version,
                source: Some(source.to_string()),
                path: Some(path.to_string_lossy().into_owned()),
            }
        }
        None => DependencyInfo {
            name: name.to_string(),
            found: false,
            version: None,
            source: None,
            path: None,
        },
    }
}

#[tauri::command]
pub async fn engine_status(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let settings = state.settings.lock().map_err(|_| "settings locked".to_string())?.clone();
    let ytdlp_found = ytdlp::locate_ytdlp(settings.ytdlp_path.as_deref(), &state.data_dir);
    let ffmpeg_found = ytdlp::locate_ffmpeg(settings.ffmpeg_path.as_deref(), &state.data_dir);
    Ok(EngineStatus {
        ytdlp: dependency_info(
            "yt-dlp",
            ytdlp_found,
            ytdlp::managed_binary(&state.data_dir),
            settings.ytdlp_path.as_deref(),
        ),
        ffmpeg: dependency_info(
            "ffmpeg",
            ffmpeg_found,
            ytdlp::managed_ffmpeg(&state.data_dir),
            settings.ffmpeg_path.as_deref(),
        ),
    })
}

#[tauri::command]
pub async fn engine_install(state: State<'_, AppState>, name: String) -> Result<(), String> {
    match name.as_str() {
        "ytdlp" => {
            ytdlp::install_ytdlp(&state.data_dir).await.map_err(|e| e.to_string())?;
            Ok(())
        }
        "ffmpeg" => {
            #[cfg(target_os = "linux")]
            {
                return Err("On Linux install FFmpeg from your distro, for example apt install ffmpeg.".to_string());
            }
            #[cfg(not(target_os = "linux"))]
            {
                ytdlp::install_ffmpeg(&state.data_dir).await.map_err(|e| e.to_string())?;
                Ok(())
            }
        }
        other => Err(format!("Unknown engine '{other}'.")),
    }
}

#[tauri::command]
pub async fn reveal_in_folder(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if !target.exists() {
        return Err("That file no longer exists.".to_string());
    }
    if cfg!(target_os = "windows") {
        // explorer /select highlights the file; fall back to opening the parent.
        let mut explorer = Command::new("explorer");
        explorer.arg("/select,").arg(&target);
        ytdlp::hide_console_std(&mut explorer);
        let status = explorer
            .status()
            .map_err(|e| format!("Could not open the folder: {e}"))?;
        if status.success() {
            return Ok(());
        }
    }
    let parent = target.parent().map(|p| p.to_path_buf()).unwrap_or(target);
    open::that_detached(parent).map_err(|e| format!("Could not open the folder: {e}"))
}

#[tauri::command]
pub async fn open_with_default(path: String) -> Result<(), String> {
    let target = std::path::PathBuf::from(&path);
    if !target.exists() {
        return Err("That file no longer exists.".to_string());
    }
    open::that_detached(target).map_err(|e| format!("Could not open the file: {e}"))
}

#[tauri::command]
pub async fn default_output_dir() -> Result<String, String> {
    Ok(dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string()))
}
