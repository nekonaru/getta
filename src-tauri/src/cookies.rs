//! cookies.txt buckets: local integrity checks. Reads the file, reports
//! cookie counts, distinct sites and expiry health. No network, no
//! credential access, no decryption.

use serde::Serialize;

use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub struct CookieReport {
    pub ok: bool,
    pub total: usize,
    pub expired: usize,
    pub sites: Vec<String>,
    pub soonest_expiry: Option<i64>,
    pub error: Option<String>,
}

pub fn check_file(path: &Path) -> CookieReport {
    let raw = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            return CookieReport {
                ok: false,
                total: 0,
                expired: 0,
                sites: Vec::new(),
                soonest_expiry: None,
                error: Some(format!("unreadable: {e}")),
            }
        }
    };

    let mut total = 0usize;
    let mut sites = std::collections::BTreeSet::new();
    let mut expired = 0usize;
    let mut soonest: Option<i64> = None;
    let mut malformed = 0usize;

    for (index, raw_line) in raw.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        // #HttpOnly_ marks an HttpOnly cookie in the Netscape format.
        let line = line.strip_prefix("#HttpOnly_").unwrap_or(line);
        if line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 7 {
            malformed += 1;
            continue;
        }
        let domain = fields[0].trim().trim_start_matches('.').to_ascii_lowercase();
        if domain.is_empty() || !domain.contains('.') {
            malformed += 1;
            continue;
        }
        let expiry = match fields[4].trim().parse::<i64>() {
            Ok(value) => value,
            Err(_) => {
                malformed += 1;
                continue;
            }
        };
        total += 1;
        sites.insert(domain);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if expiry != 0 {
            if expiry <= now {
                expired += 1;
            } else {
                soonest = Some(soonest.map_or(expiry, |cur| cur.min(expiry)));
            }
        }
        let _ = index;
    }

    let ok = malformed == 0 && total > 0;
    let error = if total == 0 {
        Some("no cookies found".to_string())
    } else if malformed > 0 {
        Some(format!("{malformed} malformed line(s)"))
    } else {
        None
    };

    CookieReport {
        ok,
        total,
        expired,
        sites: sites.into_iter().collect(),
        soonest_expiry: soonest,
        error,
    }
}

pub fn check_path(path: &str) -> CookieReport {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return CookieReport {
            ok: false,
            total: 0,
            expired: 0,
            sites: Vec::new(),
            soonest_expiry: None,
            error: Some("no file chosen".to_string()),
        };
    }
    check_file(Path::new(trimmed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_valid_and_expired() {
        let dir = std::env::temp_dir().join(format!("getta-cookie-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("cookies.txt");
        std::fs::write(
            &file,
            "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t4102444800\tSID\tabc\n.reddit.com\tTRUE\t/\tFALSE\t4102444800\tsess\tdef\n",
        )
        .unwrap();
        let report = check_file(&file);
        assert!(report.ok);
        assert_eq!(report.total, 2);
        assert_eq!(report.sites, vec!["reddit.com".to_string(), "youtube.com".to_string()]);
        assert_eq!(report.expired, 0);
        assert!(report.soonest_expiry.is_some());
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn flags_malformed_lines() {
        let dir = std::env::temp_dir().join(format!("getta-cookie-test-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("cookies.txt");
        std::fs::write(&file, "not-a-cookie-line\n").unwrap();
        let report = check_file(&file);
        assert!(!report.ok);
        assert_eq!(report.total, 0);
        assert!(report.error.is_some());
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_dir(&dir);
    }
}
