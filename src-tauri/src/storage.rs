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
pub fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let text = serde_json::to_string_pretty(value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Load a JSON file or return `T::default()` when it does not exist or is
/// corrupt. A corrupt file is moved aside once so the next read starts clean.
pub fn read_json_or_default<T>(path: &Path) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    match read_json(path) {
        Ok(value) => value,
        Err(e) if e.kind() == io::ErrorKind::NotFound => T::default(),
        Err(e) => {
            tracing::warn!("settings/state file unreadable ({e}); starting from defaults");
            let quarantine = path.with_extension("bad");
            let _ = fs::rename(path, &quarantine);
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
        assert_eq!(value, serde_json::Value::Object(Default::default()));
        std::fs::remove_dir_all(&dir).ok();
    }
}
