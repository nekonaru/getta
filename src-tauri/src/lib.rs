//! Getta library entry point.
//!
//! Getta is an original implementation. Nothing in this crate is derived
//! from any other downloader project.

mod commands;
mod history;
mod models;
mod platform;
mod queue;
mod recovery;
mod settings;
mod storage;
mod tray;
mod ytdlp;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

pub struct AppState {
    pub data_dir: PathBuf,
    pub settings: Mutex<settings::AppSettings>,
    pub queue: queue::QueueManager,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            // A second instance carrying a link forwards it into the running app.
            if let Some(url) = argv.iter().find(|arg| {
                arg.starts_with("http://")
                    || arg.starts_with("https://")
                    || arg.starts_with("magnet:")
                    || arg.starts_with("p2p:")
                    || arg.starts_with("getta://")
            }) {
                use tauri::Emitter;
                let _ = app.emit("deep-link-request", url.clone());
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state() == ShortcutState::Pressed {
                        use tauri::Emitter;
                        let _ = app.emit("global-download-requested", ());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec![])))
        .plugin(tauri_plugin_clipboard_manager::init())
        .on_window_event(|window, event| {
            // Optional behavior: hide to tray instead of quitting on X.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .app_handle()
                    .try_state::<AppState>()
                    .and_then(|state| state.settings.lock().ok().map(|s| s.close_to_tray))
                    .unwrap_or(false);
                if close_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            let data_dir = storage::data_dir();
            std::fs::create_dir_all(&data_dir).ok();
            std::fs::create_dir_all(data_dir.join("bin")).ok();
            let settings = settings::load(&data_dir);
            app.manage(AppState {
                data_dir: data_dir.clone(),
                settings: Mutex::new(settings),
                queue: queue::QueueManager::new(data_dir),
            });
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("Getta");
            }
            let language = app
                .try_state::<AppState>()
                .and_then(|state| state.settings.lock().ok().map(|s| s.language.clone()))
                .unwrap_or_else(|| "en".to_string());
            let (show_label, quit_label) = tray::tray_labels(&language);
            tray::setup(app, &show_label, &quit_label)?;

            // Register the global fetch shortcut (CmdOrCtrl+Shift+D).
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
                match "CmdOrCtrl+Shift+D".parse::<Shortcut>() {
                    Ok(shortcut) => {
                        if let Err(e) = app.global_shortcut().register(shortcut) {
                            tracing::warn!("global shortcut unavailable: {e}");
                        }
                    }
                    Err(e) => tracing::warn!("invalid shortcut: {e}"),
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze_url,
            commands::run_tool,
            commands::start_download,
            commands::pause_download,
            commands::resume_download,
            commands::cancel_download,
            commands::retry_download,
            commands::remove_download,
            commands::clear_finished,
            commands::pause_all,
            commands::resume_all,
            commands::queue_snapshot,
            commands::move_item,
            commands::get_recovery_items,
            commands::restore_recovery,
            commands::dismiss_recovery,
            commands::history_list,
            commands::history_clear,
            commands::history_remove,
            commands::settings_get,
            commands::settings_update,
            commands::settings_reset,
            commands::engine_status,
            commands::engine_install,
            commands::reveal_in_folder,
            commands::open_with_default,
            commands::default_output_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Getta");
}

