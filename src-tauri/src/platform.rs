//! URL classification. Native extractors are not wired up yet; every platform
//! routes through the yt-dlp engine for now. Detection still tells the UI
//! which site a link belongs to and guards inputs the engine cannot handle.

use url::Url;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlatformKind {
    Media,
    File,
}

pub struct Platform {
    pub id: String,
    pub display_name: String,
    pub kind: PlatformKind,
    pub supported: bool,
}

struct Matcher {
    hosts: &'static [&'static str],
    id: &'static str,
    name: &'static str,
}

const MEDIA_MATCHERS: &[Matcher] = &[
    Matcher { hosts: &["youtube.com", "www.youtube.com", "m.youtube.com", "youtu.be", "youtube-nocookie.com", "music.youtube.com"], id: "youtube", name: "YouTube" },
    Matcher { hosts: &["instagram.com", "www.instagram.com"], id: "instagram", name: "Instagram" },
    Matcher { hosts: &["tiktok.com", "www.tiktok.com", "vm.tiktok.com", "vt.tiktok.com"], id: "tiktok", name: "TikTok" },
    Matcher { hosts: &["twitter.com", "www.twitter.com", "x.com", "www.x.com", "vxtwitter.com", "fixvx.com"], id: "x", name: "X / Twitter" },
    Matcher { hosts: &["reddit.com", "www.reddit.com", "old.reddit.com", "v.redd.it", "redd.it"], id: "reddit", name: "Reddit" },
    Matcher { hosts: &["twitch.tv", "www.twitch.tv", "clips.twitch.tv"], id: "twitch", name: "Twitch" },
    Matcher { hosts: &["vimeo.com", "www.vimeo.com"], id: "vimeo", name: "Vimeo" },
    Matcher { hosts: &["bilibili.com", "www.bilibili.com", "b23.tv", "t.bilibili.com"], id: "bilibili", name: "Bilibili" },
    Matcher { hosts: &["soundcloud.com", "www.soundcloud.com", "m.soundcloud.com"], id: "soundcloud", name: "SoundCloud" },
    Matcher { hosts: &["pinterest.com", "www.pinterest.com", "pin.it", "pinterest.co.uk"], id: "pinterest", name: "Pinterest" },
    Matcher { hosts: &["threads.net", "www.threads.net", "threads.com", "www.threads.com"], id: "threads", name: "Threads" },
    Matcher { hosts: &["bsky.app", "bsky.social"], id: "bluesky", name: "Bluesky" },
    Matcher { hosts: &["douyin.com", "www.douyin.com", "iesdouyin.com"], id: "douyin", name: "Douyin" },
    Matcher { hosts: &["facebook.com", "www.facebook.com", "fb.watch", "m.facebook.com"], id: "facebook", name: "Facebook" },
    Matcher { hosts: &["linkedin.com", "www.linkedin.com"], id: "linkedin", name: "LinkedIn" },
    Matcher { hosts: &["dailymotion.com", "www.dailymotion.com"], id: "dailymotion", name: "Dailymotion" },
    Matcher { hosts: &["bitchute.com", "www.bitchute.com"], id: "bitchute", name: "BitChute" },
    Matcher { hosts: &["rumble.com", "www.rumble.com"], id: "rumble", name: "Rumble" },
    Matcher { hosts: &["odysee.com", "www.odysee.com"], id: "odysee", name: "Odysee" },
    Matcher { hosts: &["kick.com", "www.kick.com"], id: "kick", name: "Kick" },
];

const DIRECT_FILE_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso", "dmg", "pkg", "exe", "msi", "appimage", "deb", "rpm",
    "pdf", "epub", "mobi", "mp3", "m4a", "flac", "ogg", "opus", "wav", "mp4", "mkv", "webm", "mov", "avi",
    "jpg", "jpeg", "png", "gif", "webp", "svg", "avif", "ico", "apk", "jar", "bin", "rom", "torrent",
];

pub fn detect(raw: &str) -> Platform {
    let trimmed = raw.trim();

    if trimmed.starts_with("magnet:") || trimmed.to_ascii_lowercase().ends_with(".torrent") {
        return Platform {
            id: "magnet".to_string(),
            display_name: "Torrent / Magnet".to_string(),
            kind: PlatformKind::File,
            supported: false,
        };
    }

    let Ok(url) = Url::parse(trimmed) else {
        return Platform {
            id: "unknown".to_string(),
            display_name: "Unknown".to_string(),
            kind: PlatformKind::File,
            supported: false,
        };
    };

    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Platform {
            id: "unknown".to_string(),
            display_name: "Unknown".to_string(),
            kind: PlatformKind::File,
            supported: false,
        };
    }

    let host = url.host_str().unwrap_or("").trim_start_matches("www.").to_ascii_lowercase();

    for matcher in MEDIA_MATCHERS {
        let hit = matcher
            .hosts
            .iter()
            .any(|h| host == *h || host.ends_with(&format!(".{h}")));
        if hit {
            return Platform {
                id: matcher.id.to_string(),
                display_name: matcher.name.to_string(),
                kind: PlatformKind::Media,
                supported: true,
            };
        }
    }

    let path = url.path().to_ascii_lowercase();
    let looks_like_file = DIRECT_FILE_EXTENSIONS
        .iter()
        .any(|ext| path.ends_with(&format!(".{ext}")));
    if looks_like_file {
        return Platform {
            id: "direct".to_string(),
            display_name: "Direct file".to_string(),
            kind: PlatformKind::File,
            supported: true,
        };
    }

    Platform {
        id: "generic".to_string(),
        display_name: "Web".to_string(),
        kind: PlatformKind::Media,
        supported: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_youtube() {
        let platform = detect("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(platform.id, "youtube");
        assert_eq!(platform.display_name, "YouTube");
        assert!(platform.supported);
    }

    #[test]
    fn detects_magnet_and_direct_file() {
        assert_eq!(detect("magnet:?xt=urn:btih:abc123").id, "magnet");
        assert_eq!(detect("https://example.com/files/archive.zip").id, "direct");
        assert!(detect("https://example.com/watch/page").supported);
        assert_eq!(detect("https://example.com/watch/page").id, "generic");
        assert!(!detect("ftp://example.com/file.zip").supported);
        assert!(!detect("not a url").supported);
    }

    #[test]
    fn host_suffix_matching_needs_dot() {
        assert_eq!(detect("https://evilyoutube.com/watch?v=x").id, "generic");
        assert_eq!(detect("https://sub.youtube.com/watch?v=x").id, "youtube");
    }
}
