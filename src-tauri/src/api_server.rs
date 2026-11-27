//! Optional loopback-only HTTP API. Disabled by default. When enabled a
//! random token is required on every request; the server never binds to a
//! public interface.

use tauri::{Emitter, Manager};

use crate::models::StartOptions;

pub async fn serve(app: tauri::AppHandle, port: u16, token: String) {
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::warn!("local API could not start on port {port}: {e}");
            return;
        }
    };
    static CONN_SEM: std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>> =
        std::sync::OnceLock::new();
    let sem = CONN_SEM.get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(8)));
    loop {
        let Ok((mut stream, _)) = listener.accept().await else {
            continue;
        };
        let Ok(permit) = sem.clone().try_acquire_owned() else {
            // Too many open connections. Close this one immediately.
            continue;
        };
        let app = app.clone();
        let token = token.clone();
        tauri::async_runtime::spawn(async move {
            let _permit = permit;
            handle(&app, &mut stream, &token).await;
        });
    }
}

async fn handle(app: &tauri::AppHandle, stream: &mut tokio::net::TcpStream, token: &str) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buffer = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];
    // Read until the header terminator, then keep reading the declared body.
    loop {
        let Ok(Ok(size)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read(&mut chunk),
        )
        .await
        else {
            return;
        };
        if size == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..size]);
        if buffer.len() > 65536 {
            return;
        }
        if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&buffer[..pos]).to_ascii_lowercase();
            let needed = header
                .lines()
                .find_map(|line| {
                    line.strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0)
                .min(65536);
            if buffer.len() >= pos + 4 + needed {
                break;
            }
        }
    }
    let request = String::from_utf8_lossy(&buffer).to_string();
    let authorized = request.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("authorization:")
            && line.contains(token)
    });
    let (status, body): (u16, String) = if !authorized {
        (401, "{\"error\":\"unauthorized\"}".to_string())
    } else if request.starts_with("GET /status") {
        let count = app
            .try_state::<crate::AppState>()
            .map(|state| state.queue.snapshot().len())
            .unwrap_or(0);
        (200, format!("{{\"queue\":{count}}}"))
    } else if request.starts_with("POST /add") {
        let payload = request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or("");
        let url = serde_json::from_str::<serde_json::Value>(payload)
            .ok()
            .and_then(|value| value.get("url").and_then(|u| u.as_str()).map(str::to_string));
        let Some(url) = url else {
            return respond(stream, 400, "{\"error\":\"missing url\"}").await;
        };
        let options = StartOptions {
            url: url.clone(),
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
            filename_template: None,
        };
        let added = app
            .try_state::<crate::AppState>()
            .and_then(|state| {
                state
                    .queue
                    .start_counted(app, &options.url, &options)
                    .map(|(_, inserted)| inserted)
                    .ok()
            })
            .unwrap_or(false);
        if added {
            (200, "{\"queued\":true}".to_string())
        } else {
            (200, "{\"queued\":false}".to_string())
        }
    } else {
        (404, "{\"error\":\"not found\"}".to_string())
    };
    respond(stream, status, &body).await;
}

async fn respond(stream: &mut tokio::net::TcpStream, status: u16, body: &str) {
    use tokio::io::AsyncWriteExt;
    let reason = if status == 200 {
        "OK"
    } else {
        "Unauthorized"
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}
