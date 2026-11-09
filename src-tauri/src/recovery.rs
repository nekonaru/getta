//! Interrupted download recovery. When Getta is closed or killed while a
//! download is active, the intent stays here and is offered back on launch.
//! yt-dlp picks unfinished transfers up from `.part` files automatically.

use std::io;
use std::path::{Path, PathBuf};

use crate::models::RecoveryEntry;
use crate::storage;

fn recovery_path(data_dir: &Path) -> PathBuf {
    data_dir.join("recovery.json")
}

pub fn push(data_dir: &Path, entry: RecoveryEntry) -> io::Result<()> {
    let mut entries = list(data_dir);
    // Same URL with different options is a different attempt; keep them all.
    entries.push(entry);
    storage::write_json_atomic(&recovery_path(data_dir), &entries)
}

pub fn remove(data_dir: &Path, id: u64) -> io::Result<()> {
    let entries = list(data_dir).into_iter().filter(|e| e.id != id).collect::<Vec<_>>();
    storage::write_json_atomic(&recovery_path(data_dir), &entries)
}

pub fn list(data_dir: &Path) -> Vec<RecoveryEntry> {
    storage::read_json_or_default(&recovery_path(data_dir))
}

pub fn clear(data_dir: &Path) -> io::Result<()> {
    storage::write_json_atomic(&recovery_path(data_dir), &Vec::<RecoveryEntry>::new())
}
