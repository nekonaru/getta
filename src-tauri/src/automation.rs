//! Link resolution and watch-folder automation.

use std::path::PathBuf;

/// Known shortener hosts we are willing to follow. Anything else is left
/// untouched so a random redirect cannot change the target silently.
const SHORTENERS: &[&str] = &[
    "bit.ly",
    "tinyurl.com",
    "goo.gl",
    "t.co",
    "is.gd",
    "ow.ly",
    "buff.ly",
    "cutt.ly",
    "rebrand.ly",
    "s.id",
];

pub async fn resolve(raw: &str) -> String {
    let Ok(url) = url::Url::parse(raw) else {
        return raw.to_string();
    };
    let host = url
        .host_str()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    if !SHORTENERS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
    {
        return raw.to_string();
    }
    let client = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(_) => return raw.to_string(),
    };
    match client.head(raw).send().await {
        Ok(response) => response.url().to_string(),
        Err(_) => raw.to_string(),
    }
}

/// Poll a watched folder for `.txt` files, enqueue every http(s) link inside
/// and rename the file to `.done.txt` so it is not processed twice.
pub async fn poll_watch_folder(app: tauri::AppHandle) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let (folder, enabled) = app
            .try_state::<crate::AppState>()
            .and_then(|state| {
                state.settings.lock().ok().map(|s| {
                    (
                        s.watch_folder.trim().to_string(),
                        !s.watch_folder.trim().is_empty(),
                    )
                })
            })
            .unwrap_or_default();
        if !enabled {
            continue;
        }
        let dir = PathBuf::from(&folder);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                continue;
            }
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .ends_with(".done.txt")
            {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let mut queued = 0usize;
            for token in text.split_whitespace() {
                let token = token.trim().trim_matches(['\'', '"', ',', ';']);
                if !token.starts_with("http://") && !token.starts_with("https://") {
                    continue;
                }
                let options = crate::models::StartOptions {
                    url: token.to_string(),
                    download_mode: "auto".to_string(),
                    quality: "best".to_string(),
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
                };
                if let Some(state) = app.try_state::<crate::AppState>() {
                    if state
                        .queue
                        .start_counted(&app, &options.url, &options)
                        .map(|(_, inserted)| inserted)
                        .unwrap_or(false)
                    {
                        queued += 1;
                    }
                }
            }
            if queued > 0 {
                let label = format!("{queued} links from {}", path.display());
                if let Some(state) = app.try_state::<crate::AppState>() {
                    crate::features::audit(&state.data_dir, "watch_folder", &label);
                }
                let done = path.with_extension("done.txt");
                let _ = std::fs::rename(&path, &done);
            }
        }
    }
}
