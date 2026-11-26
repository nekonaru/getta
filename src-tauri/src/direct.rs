//! Helpers for native direct-file downloads.

use url::Url;

/// Derive a safe file name from a URL path or its query string.
pub fn filename_from_url(raw: &str) -> String {
    let fallback = format!("file-{}", crate::queue::now_secs());
    let Ok(url) = Url::parse(raw) else {
        return fallback;
    };
    let candidate = url
        .path_segments()
        .and_then(|segments| segments.filter(|s| !s.is_empty()).last())
        .map(|s| s.to_string())
        .filter(|s| s.contains('.'))
        .or_else(|| {
            url.query_pairs()
                .find(|(key, _)| key.eq_ignore_ascii_case("file") || key.eq_ignore_ascii_case("filename"))
                .map(|(_, value)| value.into_owned())
        })
        .unwrap_or_else(|| fallback.clone());
    let decoded = percent_decode(&candidate);
    let safe: String = decoded
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = safe.trim().trim_matches('.');
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed.chars().take(180).collect()
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push(high * 16 + low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_from_url_uses_path() {
        assert_eq!(
            filename_from_url("https://cdn.example.com/files/report%20final.pdf?token=abc"),
            "report final.pdf"
        );
    }
}
