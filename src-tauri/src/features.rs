//! Extra local features: library metadata, backup, audit log, health checks
//! and offline exports. Every operation stays inside the app data directory
//! unless the user explicitly picks a path.

use std::path::{Path, PathBuf};

use crate::history;
use crate::storage;

fn audit_path(data_dir: &Path) -> PathBuf {
    data_dir.join("audit.jsonl")
}

pub fn audit(data_dir: &Path, action: &str, detail: &str) {
    let line = serde_json::json!({
        "at": crate::queue::now_secs(),
        "action": action,
        "detail": detail,
    })
    .to_string();
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_path(data_dir))
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

pub fn read_audit(data_dir: &Path) -> Vec<String> {
    std::fs::read_to_string(audit_path(data_dir))
        .unwrap_or_default()
        .lines()
        .rev()
        .take(500)
        .map(str::to_string)
        .collect()
}

pub fn patch_history(
    data_dir: &Path,
    id: &str,
    tags: Option<Vec<String>>,
    rating: Option<u8>,
    note: Option<String>,
    favorite: Option<bool>,
    watch_later: Option<bool>,
) -> Result<bool, String> {
    let _guard = crate::history::lock();
    let mut entries = history::list(data_dir);
    let Some(entry) = entries.iter_mut().find(|e| e.id == id) else {
        return Ok(false);
    };
    if let Some(value) = tags {
        entry.tags = value
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .take(20)
            .collect();
    }
    if let Some(value) = rating {
        entry.rating = value.min(5);
    }
    if let Some(value) = note {
        entry.note = value.chars().take(2000).collect();
    }
    if let Some(value) = favorite {
        entry.favorite = value;
    }
    if let Some(value) = watch_later {
        entry.watch_later = value;
    }
    storage::write_json_atomic(&data_dir.join("history.json"), &entries)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// Apply a name template to selected history rows and move the files on disk.
pub fn batch_rename(data_dir: &Path, ids: &[String], template: &str) -> Result<usize, String> {
    let _guard = crate::history::lock();
    let mut entries = history::list(data_dir);
    let mut changed = 0usize;
    for entry in entries.iter_mut() {
        if !ids.contains(&entry.id) {
            continue;
        }
        let Some(old) = entry.file_path.clone() else { continue };
        let source = PathBuf::from(&old);
        let Some(parent) = source.parent() else { continue };
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        let stem = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let rendered = template
            .replace("{title}", &entry.title)
            .replace("{platform}", &entry.platform)
            .replace("{id}", &sanitize_segment(&entry.id))
            .replace("{old}", &stem)
            .replace("{ext}", &ext);
        let safe = sanitize_segment(&rendered);
        if safe.is_empty() {
            continue;
        }
        let mut target = parent.join(format!("{safe}.{ext}"));
        let mut counter = 1;
        while target.exists() && target != source {
            target = parent.join(format!("{safe} ({counter}).{ext}"));
            counter += 1;
        }
        if target == source {
            continue;
        }
        if std::fs::rename(&source, &target).is_ok() {
            entry.file_path = Some(target.to_string_lossy().into_owned());
            changed += 1;
        }
    }
    storage::write_json_atomic(&data_dir.join("history.json"), &entries)
        .map_err(|e| e.to_string())?;
    Ok(changed)
}

fn sanitize_segment(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')') {
                c
            } else {
                '_'
            }
        })
        .collect();
    cleaned.trim().trim_matches('.').chars().take(140).collect()
}

#[derive(serde::Serialize)]
pub struct HealthReport {
    pub data_dir: String,
    pub output_dir: String,
    pub disk_free_bytes: u64,
    pub disk_total_bytes: u64,
    pub enough_space: bool,
    pub output_writable: bool,
    pub history_count: usize,
    pub audit_count: usize,
}

pub fn health(data_dir: &Path, output_dir: &Path, needed_bytes: u64) -> HealthReport {
    let (free, total) = disk_space(output_dir);
    let probe = output_dir.join(".getta-write-test");
    let writable = std::fs::write(&probe, b"ok").is_ok();
    if writable {
        let _ = std::fs::remove_file(&probe);
    }
    HealthReport {
        data_dir: data_dir.to_string_lossy().into_owned(),
        output_dir: output_dir.to_string_lossy().into_owned(),
        disk_free_bytes: free,
        disk_total_bytes: total,
        // Zero means the platform cannot report free space. Treat it as
        // unknown rather than blocking every download.
        enough_space: free == 0 || free >= needed_bytes.max(512 * 1024 * 1024),
        output_writable: writable,
        history_count: history::list(data_dir).len(),
        audit_count: read_audit(data_dir).len(),
    }
}

#[cfg(windows)]
fn disk_space(path: &Path) -> (u64, u64) {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut free: u64 = 0;
    let mut total: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free as *mut u64,
            &mut total as *mut u64,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 { (0, 0) } else { (free, total) }
}

#[cfg(windows)]
extern "system" {
    #[link_name = "GetDiskFreeSpaceExW"]
    fn GetDiskFreeSpaceExW(
        directory_name: *const u16,
        free_bytes_available: *mut u64,
        total_bytes: *mut u64,
        total_bytes_free: *mut u64,
    ) -> i32;
}

#[cfg(not(windows))]
fn disk_space(path: &Path) -> (u64, u64) {
    let _ = path;
    (0, 0)
}

pub fn export_library_html(data_dir: &Path, dest: &Path) -> Result<(), String> {
    let entries = history::list(data_dir);
    let mut html = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Getta Library</title><style>body{font:14px system-ui;background:#faf8f3;color:#1b1b1b;max-width:960px;margin:2rem auto;padding:0 1rem}h1{font-size:1.6rem}.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:12px}.card{border:1px solid #ddd;border-radius:10px;padding:12px;background:#fff}a{color:#007a5e}</style></head><body><h1>Getta Library</h1><div class=\"grid\">",
    );
    for entry in entries.iter().filter(|e| e.success) {
        let title = html_escape(&entry.title);
        let url = html_escape(&entry.url);
        let platform = html_escape(&entry.platform);
        html.push_str(&format!(
            "<div class=\"card\"><strong>{title}</strong><br><span>{platform}</span><br><a href=\"{url}\">{url}</a></div>"
        ));
    }
    html.push_str("</div></body></html>");
    storage::write_text_atomic(dest, &html).map_err(|e| e.to_string())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn create_backup(data_dir: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for name in ["settings.json", "history.json", "follows.json", "recovery.json"] {
        let path = data_dir.join(name);
        if path.is_file() {
            zip.start_file(name, options)
                .map_err(|e| e.to_string())?;
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            use std::io::Write;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore_backup(data_dir: &Path, source: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(source).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut restored = 0usize;
    for name in ["settings.json", "history.json", "follows.json", "recovery.json"] {
        if let Ok(mut entry) = archive.by_name(name) {
            let mut bytes = Vec::new();
            use std::io::Read;
            entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            let temp = data_dir.join(format!("{name}.restore-tmp"));
            std::fs::write(&temp, &bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&temp, data_dir.join(name)).map_err(|e| e.to_string())?;
            restored += 1;
        }
    }
    Ok(restored)
}


fn profile_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("profiles")
}

pub fn list_profiles(data_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(profile_dir(data_dir)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.path().extension().and_then(|x| x.to_str()).filter(|x| *x == "json").map(|_| {
            e.path().file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string()
        }))
        .filter(|s| !s.is_empty())
        .collect();
    names.sort();
    names
}

pub fn save_profile(data_dir: &Path, name: &str, settings: &crate::settings::AppSettings) -> Result<(), String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') { c } else { '_' })
        .collect();
    if safe.trim().is_empty() {
        return Err("Profile name is empty.".to_string());
    }
    let dir = profile_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    storage::write_json_atomic(&dir.join(format!("{safe}.json")), settings)
        .map_err(|e| e.to_string())
}

pub fn load_profile(data_dir: &Path, name: &str) -> Result<crate::settings::AppSettings, String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_') { c } else { '_' })
        .collect();
    storage::read_json(&profile_dir(data_dir).join(format!("{safe}.json")))
        .map_err(|e| format!("Could not load profile: {e}"))
}


/// Create a ZIP with diagnostic data only. Tokens and cookies are excluded.
pub fn diagnostic_bundle(data_dir: &Path, dest: &Path, health: &HealthReport) -> Result<(), String> {
    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    use std::io::Write;
    for (name, text) in [
        ("health.json", serde_json::to_string_pretty(health).unwrap_or_default()),
        ("audit.jsonl", std::fs::read_to_string(audit_path(data_dir)).unwrap_or_default()),
        ("version.txt", format!("getta {}", env!("CARGO_PKG_VERSION"))),
    ] {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
