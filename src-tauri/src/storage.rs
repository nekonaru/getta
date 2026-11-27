//! Small file helpers. Everything user-facing is written atomically so a
//! crash mid-write can never wipe state.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GETTA_DATA_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    if let Some(base) = dirs::data_dir() {
        return base.join("getta");
    }
    PathBuf::from(".getta")
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> io::Result<T> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid JSON in {path:?}: {e}")))
}

/// Write `value` to `path` atomically via a sibling temp file and rename.
/// The temp file name is unique per call so concurrent writers never clobber
/// each other.
pub fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let suffix = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("tmp-{}-{nanos}-{suffix}", std::process::id()));
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    write_bytes_atomic(path, tmp, text.as_bytes())
}

/// Write UTF-8 text atomically via a sibling temp file and rename.
pub fn write_text_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let suffix = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = path.with_extension(format!("tmp-{}-{nanos}-{suffix}", std::process::id()));
    write_bytes_atomic(path, tmp, text.as_bytes())
}

/// Flush the temp file to disk before the rename. A rename can be durable
/// while its data is not, which used to leave an empty state file after a
/// sudden power loss.
fn write_bytes_atomic(path: &Path, tmp: PathBuf, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = fs::File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Load a JSON file or return `T::default()` when it does not exist. When the
/// file is corrupt, keep it as `.bad` and salvage the records that still
/// deserialize instead of losing the whole file.
pub fn read_json_or_default<T>(path: &Path) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    match read_json(path) {
        Ok(value) => value,
        Err(e) if e.kind() == io::ErrorKind::NotFound => T::default(),
        Err(e) => {
            tracing::warn!("state file unreadable ({e}); recovering valid records");
            let text = fs::read_to_string(path).unwrap_or_default();
            let quarantine = path.with_extension("bad");
            let _ = fs::rename(path, &quarantine);
            if let Ok(serde_json::Value::Array(items)) = serde_json::from_str::<serde_json::Value>(&text) {
                let valid: Vec<serde_json::Value> = items
                    .iter()
                    .filter(|item| {
                        serde_json::from_value::<T>(serde_json::Value::Array(vec![(*item).clone()])).is_ok()
                    })
                    .cloned()
                    .collect();
                if !valid.is_empty() || items.is_empty() {
                    if let Ok(recovered) =
                        serde_json::from_value::<T>(serde_json::Value::Array(valid))
                    {
                        let _ = write_json_atomic(path, &recovered);
                        return recovered;
                    }
                }
            }
            T::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_survives_corruption() {
        let dir = std::env::temp_dir().join(format!("getta-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");

        write_json_atomic(&path, &serde_json::json!({"a": 1})).unwrap();
        let value: serde_json::Value = read_json(&path).unwrap();
        assert_eq!(value["a"], 1);

        std::fs::write(&path, "{not json").unwrap();
        let value: serde_json::Value = read_json_or_default(&path);
        assert_eq!(value, serde_json::Value::Null);
        std::fs::remove_dir_all(&dir).ok();
    }
}
