//! Getta library entry point.
//!
//! Getta is an original implementation. Nothing in this crate is derived
//! from any other downloader project.

mod commands;
mod history;
mod models;
mod platform;
mod queue;
mod settings;
mod storage;
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
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
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
                let title = format!("Getta");
                let _ = window.set_title(&title);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze_url,
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
            commands::history_list,
            commands::history_clear,
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

