//! Batch import helpers.

use tauri::{Emitter, Manager};

use crate::models::StartOptions;

/// Read a text, JSON or HTML file and return unique http(s) links.
pub fn extract_links(path: &std::path::Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut links = Vec::new();
    for token in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ',')
    }) {
        let token = token.trim().trim_end_matches([';', ')', ']']);
        if (token.starts_with("http://") || token.starts_with("https://"))
            && !links.iter().any(|l| l == token)
        {
            links.push(token.to_string());
        }
    }
    links.truncate(500);
    Ok(links)
}

pub fn enqueue_links(app: &tauri::AppHandle, links: Vec<String>) -> usize {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return 0;
    };
    let quality = state
        .settings
        .lock()
        .map(|s| s.video_quality.clone())
        .unwrap_or_else(|_| "best".to_string());
    let mut added = 0usize;
    for url in links {
        let options = StartOptions {
            url: url.clone(),
            download_mode: "auto".to_string(),
            quality: quality.clone(),
            format_id: None,
            playlist_items: None,
            output_dir: None,
            start_at: None,
            priority: 0,
            speed_limit_mbps: 0,
            expected_sha256: None,
            group_id: None,
            after_id: None,
            preset_id: None,
            direct: false,
            filename_template: None,
        };
        if state
            .queue
            .start_counted(app, &options.url, &options)
            .map(|(_, inserted)| inserted)
            .unwrap_or(false)
        {
            added += 1;
        }
    }
    let _ = app.emit("queue-changed", state.queue.snapshot());
    added
}
