//! System tray: toggle the window on click, Show/Quit from the menu.
//! Labels follow the configured UI language.

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn tray_labels(language: &str) -> (&'static str, &'static str) {
    if language == "id" {
        ("Tampilkan Getta", "Keluar Getta")
    } else {
        ("Show Getta", "Quit Getta")
    }
}

pub fn setup(app: &tauri::App, show_label: &str, quit_label: &str) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", show_label).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", quit_label).build(app)?;
    let menu = MenuBuilder::new(app).item(&show).item(&quit).build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).expect("embedded tray icon"));
    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("Getta")
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    // The tray must outlive the setup closure; Tauri keeps it alive for the session.
    std::mem::forget(tray);
    Ok(())
}

fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
