//! Finished downloads, recorded in a single capped JSON file.

use std::path::{Path, PathBuf};

use crate::models::HistoryEntry;
use crate::storage;

const HISTORY_CAP: usize = 200;

fn history_path(data_dir: &Path) -> PathBuf {
    data_dir.join("history.json")
}

pub fn list(data_dir: &Path) -> Vec<HistoryEntry> {
    storage::read_json_or_default(&history_path(data_dir))
}

pub fn push(data_dir: &Path, entry: HistoryEntry) {
    let mut entries = list(data_dir);
    entries.insert(0, entry);
    entries.truncate(HISTORY_CAP);
    let _ = storage::write_json_atomic(&history_path(data_dir), &entries);
}

pub fn clear(data_dir: &Path) -> std::io::Result<()> {
    storage::write_json_atomic(&history_path(data_dir), &Vec::<HistoryEntry>::new())
}
