//! Link resolution and watch-folder automation.

use std::path::PathBuf;

use tauri::Manager;

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
        if !automation_allowed(&app) {
            continue;
        }
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
                record_error(&app);
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
            let label = format!("{queued} links from {}", path.display());
            if let Some(state) = app.try_state::<crate::AppState>() {
                crate::features::audit(&state.data_dir, "watch_folder", &label);
            }
            // Always move the file aside. Otherwise a file with only
            // duplicates is reprocessed every 30 seconds forever.
            let done = path.with_extension("done.txt");
            let _ = std::fs::rename(&path, &done);
        }
    }
}


/// Automation is disabled by safe mode after repeated failures.
fn automation_allowed(app: &tauri::AppHandle) -> bool {
    app.try_state::<crate::AppState>()
        .and_then(|state| state.settings.lock().ok().map(|s| !s.safe_mode))
        .unwrap_or(false)
}

fn record_error(app: &tauri::AppHandle) {
    let Some(state) = app.try_state::<crate::AppState>() else { return };
    let mut settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
    settings.automation_errors = settings.automation_errors.saturating_add(1);
    if settings.automation_errors >= 5 {
        settings.safe_mode = true;
    }
    let _ = crate::settings::save(&state.data_dir, &settings);
}

/// Poll RSS feeds and enqueue links from entries not seen before. XML is
/// parsed with simple tag scanning so no extra dependency is needed.
pub async fn poll_rss(app: tauri::AppHandle) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        if !automation_allowed(&app) {
            continue;
        }
        let feeds = app
            .try_state::<crate::AppState>()
            .and_then(|state| state.settings.lock().ok().map(|s| s.rss_feeds.clone()))
            .unwrap_or_default();
        for feed in feeds {
            let Some(text) = fetch_text(&feed.url).await else {
                record_error(&app);
                continue;
            };
            let links = extract_rss_links(&text);
            let mut new_seen = feed.seen.clone();
            let mut queued = 0usize;
            for link in links {
                if new_seen.contains(&link) {
                    continue;
                }
                let options = crate::models::StartOptions {
                    url: link.clone(),
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
                        new_seen.push(link);
                    }
                }
            }
            if new_seen.len() > 500 {
                new_seen = new_seen.split_off(new_seen.len() - 500);
            }
            if queued > 0 || new_seen != feed.seen {
                let label = format!("{}: {queued} new", feed.url);
                if let Some(state) = app.try_state::<crate::AppState>() {
                    crate::features::audit(&state.data_dir, "rss", &label);
                    let mut settings = state.settings.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(row) = settings.rss_feeds.iter_mut().find(|f| f.id == feed.id) {
                        row.seen = new_seen;
                        row.last_checked_at = crate::queue::now_secs();
                    }
                    let _ = crate::settings::save(&state.data_dir, &settings);
                }
            }
        }
    }
}

async fn fetch_text(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .ok()?;
    client.get(url).send().await.ok()?.text().await.ok()
}

fn extract_rss_links(xml: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<link") {
        rest = &rest[start + 5..];
        let Some(gt) = rest.find('>') else { break };
        rest = &rest[gt + 1..];
        let Some(end) = rest.find("</link>") else { break };
        let value = rest[..end].trim();
        let value = value
            .trim_start_matches("<![CDATA[")
            .trim_end_matches("]]>")
            .trim();
        if (value.starts_with("http://") || value.starts_with("https://"))
            && !links.iter().any(|l| l == value)
        {
            links.push(value.to_string());
        }
        rest = &rest[end + 7..];
    }
    links.truncate(50);
    links
}

/// Poll the clipboard and enqueue a link when it changes. Only known link
/// schemes are accepted, and the last text is kept to avoid duplicates.
pub async fn poll_clipboard(app: tauri::AppHandle) {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let mut last = String::new();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if !automation_allowed(&app) {
            last.clear();
            continue;
        }
        let enabled = app
            .try_state::<crate::AppState>()
            .and_then(|state| state.settings.lock().ok().map(|s| s.clipboard_monitor))
            .unwrap_or(false);
        if !enabled {
            last.clear();
            continue;
        }
        let Ok(text) = app.clipboard().read_text() else { continue };
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() || trimmed == last {
            continue;
        }
        let looks_like_link = trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("getta://")
            || trimmed.starts_with("magnet:")
            || trimmed.starts_with("p2p:");
        if !looks_like_link {
            continue;
        }
        last = trimmed.clone();
        let Some(state) = app.try_state::<crate::AppState>() else { continue };
        let options = crate::models::StartOptions {
            url: trimmed.clone(),
            download_mode: "auto".to_string(),
            quality: state.settings.lock().map(|s| s.video_quality.clone()).unwrap_or_else(|_| "best".to_string()),
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
        let _ = state.queue.start(&app, &options.url, &options);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_rss_links_reads_items() {
        let xml = "<rss><item><link>https://example.com/a.mp4</link></item><item><link><![CDATA[https://example.com/b.mp4]]></link></item></rss>";
        assert_eq!(
            extract_rss_links(xml),
            vec![
                "https://example.com/a.mp4".to_string(),
                "https://example.com/b.mp4".to_string()
            ]
        );
    }
}
