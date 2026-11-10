//! yt-dlp and FFmpeg: locating, verifying and installing the engines, plus
//! parsing yt-dlp's newline-delimited output.

use std::path::{Path, PathBuf};

use reqwest::Client;

const RELEASE_DOWNLOAD_BASE: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const SUMS_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineError {
    NotFound,
    DownloadFailed,
    ChecksumMismatch,
    VerificationFailed,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "engine not installed"),
            Self::DownloadFailed => write!(f, "download failed"),
            Self::ChecksumMismatch => write!(f, "checksum verification failed"),
            Self::VerificationFailed => write!(f, "downloaded binary does not run"),
        }
    }
}

pub fn managed_ffprobe(data_dir: &Path) -> PathBuf {
    data_dir.join("bin").join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" })
}

/// Locate ffprobe: alongside a custom FFmpeg, the managed copy, then PATH.
pub fn locate_ffprobe(ffmpeg_override: Option<&str>, data_dir: &Path) -> Option<PathBuf> {
    if let Some(custom) = ffmpeg_override {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if let Some(parent) = path.parent() {
                let sibling = parent.join(if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" });
                if sibling.is_file() {
                    return Some(sibling);
                }
            }
        }
    }
    let managed = managed_ffprobe(data_dir);
    if managed.is_file() {
        return Some(managed);
    }
    which::which("ffprobe").ok()
}

/* ---------- spawn without a console window (Windows) ---------- */

#[cfg(windows)]
pub(crate) fn hide_console_std(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
pub(crate) fn hide_console_std(_cmd: &mut std::process::Command) {}

#[cfg(windows)]
pub(crate) fn hide_console_tokio(cmd: &mut tokio::process::Command) {
    cmd.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
pub(crate) fn hide_console_tokio(_cmd: &mut tokio::process::Command) {}

fn managed_binary_name() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

pub fn managed_binary(data_dir: &Path) -> PathBuf {
    data_dir.join("bin").join(managed_binary_name())
}

/// Locate a usable yt-dlp: custom override, then the managed copy, then PATH.
pub fn locate_ytdlp(override_path: Option<&str>, data_dir: &Path) -> Option<PathBuf> {
    if let Some(custom) = override_path {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let managed = managed_binary(data_dir);
    if managed.is_file() {
        return Some(managed);
    }
    which::which("yt-dlp").ok()
}

fn ffmpeg_binary_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

pub fn managed_ffmpeg(data_dir: &Path) -> PathBuf {
    data_dir.join("bin").join(ffmpeg_binary_name())
}

/// Locate a usable FFmpeg: custom override, then the managed copy, then PATH.
pub fn locate_ffmpeg(override_path: Option<&str>, data_dir: &Path) -> Option<PathBuf> {
    if let Some(custom) = override_path {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    let managed = managed_ffmpeg(data_dir);
    if managed.is_file() {
        return Some(managed);
    }
    which::which("ffmpeg").ok()
}

fn engine_client() -> Client {
    Client::builder()
        .user_agent(concat!("getta/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("engine HTTP client must build")
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(bytes);
    hex::encode(digest)
}

fn parse_sums(sums: &str, wanted_file: &str) -> Option<String> {
    for line in sums.lines() {
        let mut parts = line.split_whitespace();
        let (Some(hash), Some(file)) = (parts.next(), parts.next()) else {
            continue;
        };
        if file.trim_start_matches("./") == wanted_file {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, EngineError> {
    let response = engine_client()
        .get(url)
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("engine download failed for {url}: {e}");
            EngineError::DownloadFailed
        })?;
    if !response.status().is_success() {
        tracing::warn!("engine download for {url} returned {}", response.status());
        return Err(EngineError::DownloadFailed);
    }
    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|_| EngineError::DownloadFailed)
}

/// Download and verify the latest yt-dlp release into `data_dir/bin`.
/// Refuses to install when the checksum cannot be verified.
pub async fn install_ytdlp(data_dir: &Path) -> Result<PathBuf, EngineError> {
    #[cfg(target_arch = "aarch64")]
    let asset = if cfg!(windows) {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp_linux_aarch64"
    };
    #[cfg(not(target_arch = "aarch64"))]
    let asset = if cfg!(windows) {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp_linux"
    };

    let sums = String::from_utf8(fetch_bytes(SUMS_URL).await?)
        .map_err(|_| EngineError::VerificationFailed)?;
    let expected = parse_sums(&sums, asset).ok_or(EngineError::ChecksumMismatch)?;

    let bytes = fetch_bytes(&format!("{RELEASE_DOWNLOAD_BASE}/{asset}")).await?;
    let actual = sha256_hex(&bytes);
    if !actual.eq_ignore_ascii_case(&expected) {
        tracing::warn!("checksum mismatch for {asset}: expected {expected}, got {actual}");
        return Err(EngineError::ChecksumMismatch);
    }

    let target = managed_binary(data_dir);
    std::fs::create_dir_all(target.parent().expect("bin dir")).map_err(|_| EngineError::DownloadFailed)?;
    std::fs::write(&target, &bytes).map_err(|_| EngineError::DownloadFailed)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target).map_err(|_| EngineError::DownloadFailed)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target, perms).map_err(|_| EngineError::DownloadFailed)?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("xattr")
            .args(["-d", "com.apple.quarantine"])
            .arg(&target)
            .output();
    }

    // Refuse an unusable binary: it must print a version.
    let mut verify = std::process::Command::new(&target);
    verify.arg("--version");
    hide_console_std(&mut verify);
    let output = verify
        .output()
        .map_err(|_| EngineError::VerificationFailed)?;
    if !output.status.success() || String::from_utf8_lossy(&output.stdout).trim().is_empty() {
        let _ = std::fs::remove_file(&target);
        return Err(EngineError::VerificationFailed);
    }

    Ok(target)
}

/// Install FFmpeg (and ffprobe) on Windows and macOS. On Linux we use the
/// distro package; the in-app installer explains how.
pub async fn install_ffmpeg(data_dir: &Path) -> Result<PathBuf, EngineError> {
    #[cfg(target_os = "linux")]
    {
        let _ = data_dir;
        return Err(EngineError::NotFound);
    }

    #[cfg(not(target_os = "linux"))]
    {
    let bin_dir = data_dir.join("bin");
    std::fs::create_dir_all(&bin_dir).map_err(|_| EngineError::DownloadFailed)?;

    #[cfg(windows)]
    let urls = vec![
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
    ];
    #[cfg(target_os = "macos")]
    let urls = vec![
        "https://evermeet.cx/ffmpeg/getrelease/zip?unlink=1",
        "https://evermeet.cx/ffprobe/getrelease/zip?unlink=1",
    ];

    for url in urls {
        let bytes = fetch_bytes(url).await?;
        let reader = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(reader).map_err(|_| EngineError::DownloadFailed)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|_| EngineError::DownloadFailed)?;
            let Some(name) = file.enclosed_name().map(|p| p.to_path_buf()) else {
                continue;
            };
            let leaf = name
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let wanted = leaf == "ffmpeg.exe" || leaf == "ffprobe.exe"
                || leaf == "ffmpeg" || leaf == "ffprobe";
            if !wanted {
                continue;
            }
            let dest = bin_dir.join(&leaf);
            let mut out = std::fs::File::create(&dest).map_err(|_| EngineError::DownloadFailed)?;
            std::io::copy(&mut file, &mut out).map_err(|_| EngineError::DownloadFailed)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest).map_err(|_| EngineError::DownloadFailed)?.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&dest, perms);
            }
        }
    }

    let ffmpeg = managed_ffmpeg(data_dir);
    if !ffmpeg.is_file() {
        return Err(EngineError::VerificationFailed);
    }
    let mut ffmpeg_check = std::process::Command::new(&ffmpeg);
    ffmpeg_check.arg("-version");
    hide_console_std(&mut ffmpeg_check);
    let output = ffmpeg_check
        .output()
        .map_err(|_| EngineError::VerificationFailed)?;
    if !output.status.success() {
        return Err(EngineError::VerificationFailed);
    }
    Ok(ffmpeg)
    }
}

pub fn version_of(path: &Path, args: &[&str]) -> Option<String> {
    let mut command = std::process::Command::new(path);
    command.args(args);
    hide_console_std(&mut command);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let version = ytdlp_version_line(&text).or_else(|| ffmpeg_version_line(&text));
    version.map(|v| v.trim().to_string())
}

fn ytdlp_version_line(text: &str) -> Option<String> {
    let first = text.lines().next()?.trim();
    if first.chars().next()?.is_ascii_digit() {
        Some(first.to_string())
    } else {
        None
    }
}

fn ffmpeg_version_line(text: &str) -> Option<String> {
    let first = text.lines().next()?;
    first.strip_prefix("ffmpeg version ").map(|rest| {
        rest.split_whitespace().next().unwrap_or(rest).to_string()
    })
}

/* ---------- progress output parsing ---------- */

#[derive(Debug, Clone, PartialEq)]
pub enum LineEvent {
    Progress {
        percent: f64,
        downloaded_bytes: Option<u64>,
        total_bytes: Option<u64>,
        speed_bps: Option<u64>,
        eta_seconds: Option<u64>,
    },
    Destination(String),
    PostProcess(String),
    PlaylistItem(u32),
    Ignored,
}

const DOWNLOAD_RE: &str = r"^\[download\]\s+([0-9]+(?:\.[0-9]+)?)%";

pub fn parse_line(line: &str) -> LineEvent {
    let line = line.trim();
    if line.is_empty() {
        return LineEvent::Ignored;
    }

    if let Some(rest) = line.strip_prefix("[download] Destination:") {
        return LineEvent::Destination(rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix("[ExtractAudio] Destination:") {
        return LineEvent::Destination(rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix("[info] Writing video subtitles to") {
        // yt-dlp prints "...to: /path" on some versions, "...to /path" on others.
        let dest = rest.trim().trim_start_matches(':').trim().to_string();
        return LineEvent::Destination(dest);
    }
    if let Some(rest) = line.strip_prefix("[Merger] Merging formats into") {
        return LineEvent::PostProcess(format!("merging {}", rest.trim_matches(['"', ' '])));
    }
    if let Some(rest) = line.strip_prefix("[Merger] Merging") {
        return LineEvent::PostProcess(format!("merging {}", rest.trim()));
    }
    if let Some(rest) = line.strip_prefix("[EmbedSubtitle]") {
        return LineEvent::PostProcess("embedding subtitles".to_string());
    }
    if line.contains("has already been downloaded") {
        return LineEvent::PostProcess("already exists".to_string());
    }

    let percent = regex::Regex::new(DOWNLOAD_RE)
        .ok()
        .and_then(|re| re.captures(line))
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok());

    if let Some(percent) = percent {
        let total_bytes = capture_unit(line, r"of\s+(?:~\s*)?([0-9]+(?:\.[0-9]+)?)\s*([KMGTPE]?i?B)");
        let speed_bps = capture_unit(line, r"at\s+([0-9]+(?:\.[0-9]+)?)\s*([KMGTPE]?i?B)/s");
        let eta_seconds = capture_eta(line);
        return LineEvent::Progress {
            percent,
            downloaded_bytes: None,
            total_bytes,
            speed_bps,
            eta_seconds,
        };
    }

    LineEvent::Ignored
}

fn capture_unit(line: &str, pattern: &str) -> Option<u64> {
    let re = regex::Regex::new(pattern).ok()?;
    let caps = re.captures(line)?;
    let value: f64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2)?.as_str().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "b" => 1_f64,
        "kb" | "kib" => 1024_f64,
        "mb" | "mib" => 1024_f64 * 1024.0,
        "gb" | "gib" => 1024_f64 * 1024.0 * 1024.0,
        "tb" | "tib" => 1024_f64 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * multiplier) as u64)
}

fn capture_eta(line: &str) -> Option<u64> {
    let re = regex::Regex::new(r"ETA\s+([0-9:]+)").ok()?;
    let value = re.captures(line)?.get(1)?.as_str();
    let parts: Vec<&str> = value.split(':').collect();
    match parts.as_slice() {
        [m, s] => Some(m.parse::<u64>().ok()? * 60 + s.parse::<u64>().ok()?),
        [h, m, s] => Some(h.parse::<u64>().ok()? * 3600 + m.parse::<u64>().ok()? * 60 + s.parse::<u64>().ok()?),
        _ => None,
    }
}

/* ---------- error classification ---------- */

pub fn classify_error(message: &str) -> (&'static str, bool) {
    let lower = message.to_ascii_lowercase();
    let retryable_rate = |m: &str| {
        m.contains("429") || m.contains("too many requests") || m.contains("rate limit") || m.contains("rate-limited")
    };
    let auth = |m: &str| {
        m.contains("sign in") || m.contains("login") || m.contains("authentication")
            || m.contains("cookie") || m.contains("403 forbidden") || m.contains("401 unauthorized")
    };
    let not_found = |m: &str| m.contains("404") || m.contains("not found") || m.contains("does not exist");
    let restricted = |m: &str| m.contains("copyright") || m.contains("private") || m.contains("blocked") || m.contains("geo");

    if retryable_rate(&lower) {
        ("rate_limited", true)
    } else if auth(&lower) {
        ("auth_required", false)
    } else if lower.contains("dpapi") {
        // Chrome 127+ app-bound encryption: cookie decryption always fails on Windows.
        ("auth_required", false)
    } else if not_found(&lower) {
        ("not_found", false)
    } else if restricted(&lower) {
        ("restricted", false)
    } else if lower.contains("ffmpeg") || lower.contains("postprocess") || lower.contains("merger") {
        ("postprocess_failed", false)
    } else {
        ("unknown", true)
    }
}

/// Trivial unit test hook: exposed for tests only.
#[cfg(test)]
#[doc(hidden)]
pub fn parse_sums_for_test(sums: &str, file: &str) -> Option<String> {
    parse_sums(sums, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_extracts_percent_and_totals() {
        let line = "[download]  45.6% of ~104.23MiB at 2.5MiB/s ETA 00:26";
        match parse_line(line) {
            LineEvent::Progress { percent, total_bytes, speed_bps, eta_seconds, .. } => {
                assert!((percent - 45.6).abs() < f64::EPSILON);
                assert_eq!(total_bytes, Some((104.23 * 1024.0 * 1024.0) as u64));
                assert_eq!(speed_bps, Some((2.5 * 1024.0 * 1024.0) as u64));
                assert_eq!(eta_seconds, Some(26));
            }
            other => panic!("expected progress, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_reads_destination() {
        match parse_line("[download] Destination: /tmp/video [abc123].mp4") {
            LineEvent::Destination(path) => assert!(path.ends_with("abc123].mp4")),
            other => panic!("expected destination, got {other:?}"),
        }
    }

    #[test]
    fn parse_line_flags_merge_step() {
        match parse_line("[Merger] Merging formats into \"/tmp/merged.mkv\"") {
            LineEvent::PostProcess(phase) => assert!(phase.starts_with("merging")),
            other => panic!("expected post-process, got {other:?}"),
        }
    }

    #[test]
    fn classify_error_maps_categories() {
        assert_eq!(classify_error("HTTP Error 429: Too Many Requests"), ("rate_limited", true));
        assert_eq!(classify_error("Sign in to confirm you are not a bot"), ("auth_required", false));
        assert_eq!(classify_error("Video unavailable 404"), ("not_found", false));
        assert_eq!(classify_error("This video is private"), ("restricted", false));
        assert_eq!(classify_error("ffmpeg postprocessing failed"), ("postprocess_failed", false));
        assert_eq!(classify_error("some odd network blip"), ("unknown", true));
    }

    #[test]
    fn sums_parser_finds_exact_asset() {
        let sums = "abc123  yt-dlp\ndef456  yt-dlp.exe\n";
        assert_eq!(parse_sums_for_test(sums, "yt-dlp.exe"), Some("def456".to_string()));
        assert_eq!(parse_sums_for_test(sums, "yt-dlp"), Some("abc123".to_string()));
        assert_eq!(parse_sums_for_test(sums, "missing"), None);
    }

    #[test]
    fn eta_and_units() {
        assert_eq!(capture_eta("x ETA 01:02:03 y"), Some(3723));
        assert_eq!(capture_eta("x ETA 07:30 y"), Some(450));
        assert_eq!(
            capture_unit("of 1.5GiB", r"of\s+(?:~\s*)?([0-9]+(?:\.[0-9]+)?)\s*([KMGTPE]?i?B)"),
            Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64)
        );
    }
}
