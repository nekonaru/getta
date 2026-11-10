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

/// Export the whole history as CSV. Unix seconds become a UTC ISO-like
/// stamp computed without a date library.
pub fn export_csv(data_dir: &Path, dest: &Path) -> std::io::Result<()> {
    let entries = list(data_dir);
    let mut out = String::from(
        "completed_at,title,platform,url,file_path,file_size_bytes,success,download_mode,quality,error\n",
    );
    for entry in entries {
        let row = [
            unix_to_utc(entry.completed_at),
            entry.title,
            entry.platform,
            entry.url,
            entry.file_path.unwrap_or_default(),
            entry.file_size_bytes.map(|n| n.to_string()).unwrap_or_default(),
            entry.success.to_string(),
            entry.download_mode.unwrap_or_default(),
            entry.quality.unwrap_or_default(),
            entry.error.unwrap_or_default(),
        ];
        out.push_str(&row.iter().map(|field| csv_cell(field)).collect::<Vec<_>>().join(","));
        out.push('\n');
    }
    std::fs::write(dest, out)
}

fn csv_cell(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn unix_to_utc(secs: i64) -> String {
    // Days since epoch; proleptic Gregorian via Howard Hinnant's algorithm.
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{year:04}-{:02}-{:02} {:02}:{:02}:{:02}", m, d, hour, minute, second)
}

pub fn clear(data_dir: &Path) -> std::io::Result<()> {
    storage::write_json_atomic(&history_path(data_dir), &Vec::<HistoryEntry>::new())
}

/// Remove one entry by id. Returns true when an entry was removed.
pub fn remove(data_dir: &Path, id: &str) -> std::io::Result<bool> {
    let entries = list(data_dir);
    let before = entries.len();
    let kept: Vec<HistoryEntry> = entries.into_iter().filter(|e| e.id != id).collect();
    let removed = kept.len() != before;
    storage::write_json_atomic(&history_path(data_dir), &kept)?;
    Ok(removed)
}
