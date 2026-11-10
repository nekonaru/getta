# Changelog

All notable changes to Getta are documented here and in `CHANGELOG.id.md`.
Newest version on top.

## v0.10.0

Getta updates itself now.

## New features

- **Auto-update.** About shows "Check for updates"; Getta downloads and installs the signed release and relaunches. On Linux it works for the AppImage.
- **Cookie buckets.** Save and reuse cookies.txt files per site; rename them, check their integrity locally, or move them to trash with restore and permanent delete.
- **Chapter markers** in the library player: jump to chapters parsed by ffprobe.
- **Duplicate detector** in the library: SHA-256 content hashing, groups duplicates and removes all but the first copy.
- **Engine self-check** at launch: Getta verifies yt-dlp and FFmpeg run and warns you when they do not.

## Improvements

- Dependency refresh: keepawake 0.6.1, sanitize-filename 0.6.0, dirs 7.

## v0.9.0

Follow channels, download rules, fuzzy everything.

## New features

- **Channel following.** Follow any public YouTube channel; Getta detects new uploads and queues them. Optional background check every 6 hours with auto-download. The existing backlog is baselined at follow time, so it never floods your queue.
- **Download rules.** Route links to folders by pattern, e.g. "youtube.com -> D:\Media\YouTube". First match wins; an explicitly chosen folder always beats a rule.
- **Fuzzy command palette.** Subsequence matching with word-boundary bonuses instead of plain substring search.
- **Export history as JSON** next to the CSV option.
- **Contact sheet** tool: spread 20 frames across the whole file into one image.

## v0.8.0

Twenty-two tools in the workbench, subtitles-only downloads, player speed.

## New features

- **Subtitles-only mode.** Download just the subtitle track (SRT/VTT) without any media, selected from the mode picker on Home.
- **Six new tools:** waveform image, boost volume, trim silence, crop to square, merge video + audio (swap the audio track), embed album art in an MP3.
- **Playback speed** in the inline library player: 0.5x to 3x.
- **Export history to CSV** from the History tab.

## v0.7.0

The workbench grows from 4 to 16 tools, grouped by type.

## New features

- **Audio:** convert to MP3/FLAC/OGG/Opus/M4A/WAV, extract the audio stream losslessly, normalize loudness to -16 LUFS, speed up or slow down playback.
- **Video:** convert containers (remux without re-encoding, or re-encode to H.264/VP9), resize to 480p-2160p, rotate, remove the audio track, grab a frame as JPG/PNG/WebP.
- **Image:** resize to a fixed width, convert to JPG/PNG/WebP.
- **Subtitles:** convert between SRT, WebVTT and ASS.
- **Typed tool params.** Every tool takes explicit parameters; output extensions are whitelisted.

## Bug fixes

- **In transit and Queued are disjoint.** Queued items no longer appear in the In transit section.
- **Self-healing queue.** A queued or retrying item starts as soon as a concurrency slot is free; it no longer waits for an external trigger.
- **Retry countdown visible.** Queued rows show the retry/backoff phase inline.
- **Progress everywhere.** Percent, bytes moved / total, speed and ETA are shown on the active row, plus the aggregate strip over the whole queue.

## v0.6.1

New name for the home input, no functional changes.

## Improvements

- **Getup.** The home input is named Getup everywhere: UI, docs and release notes.

## v0.6.0

Type a search, trim your clips, and manage the library.

## New features

- **YouTube search in Getup.** Type a plain search instead of a link; Getta shows the top results and lets you pick which ones to fetch.
- **Video Trim.** New Tools entry: cut a clip (start + length in seconds) from any video or audio file.
- **Library cleanup.** Each library entry can copy its source URL or be removed from the library; the file stays on disk.

## v0.5.0

Interrupted downloads come back, and the queue can be re-arranged.

## New features

- **Interruption recovery.** If Getta is closed or the app dies mid-download, launch shows a dialog to resume the unfinished downloads from their `.part` files.
- **Manual queue order.** Queued items have move up and move down buttons in the manifest; the scheduler spawns them in your order.
- **Split by chapters.** Optional: downloads chapters as separate files (yt-dlp).
- **Live from start.** Optional: stream live events from the beginning (yt-dlp).

## v0.4.1

Native polish and a calmer, faster experience.

## New features

- **Close to tray.** Optional: the close button hides Getta to the system tray instead of quitting; restore it anytime from the tray icon.

## Improvements

- **Engine status chip.** The sidebar chip now shows a checking state until the engine check completes, instead of a misleading steady "engine ready".
- **Window backdrop.** The native window backdrop is the warm paper tone instead of a leftover dark green.
- **Tidier strings.** Unused placeholders removed from the locale files.

## v0.4.0

A support page with every way to back Getta, and external links that work again.

## New features

- **Support Getta.** New Support page with GitHub Sponsors, Saweria, SociaBuzz and BagiBagi. Open them from the sidebar, the Support page, or About.

## Bug fixes

- **Dead external links.** Links that open the system browser now work everywhere (About, sidebar): GitHub, releases, issues and the sponsor links.

## Improvements

- **Library thumbnails.** Cards now fade in smoothly over a shimmer placeholder instead of flashing in.

## v0.3.3

Site logins work again, progress is visibly live, and edge cases from the field are fixed.

## New features

- **Cookie file support.** Pick a `cookies.txt` export in Settings, Engine; it takes precedence over reading cookies from a browser.
- **Aggregate progress bar.** The queue screen shows a single overall progress bar above the manifest, weighted by bytes where known.

## Improvements

- **Humanized errors.** Engine failures map to actionable guidance instead of raw yt-dlp stack dumps.
- **Second-instance links.** getta:// links passed to a second launch are forwarded into the running app instead of silently ignored.

## Bug fixes

- **Chrome 127+ DPAPI failure.** Edge and Chrome cookies can no longer be read by third parties; Getta now guides the user and accepts cookies.txt as the supported route.
- **Resume respects concurrency.** Resuming a paused download queues it back within the limit instead of jumping ahead.
- **Duplicate deep-link enqueue.** Starting the app via getta:// while it was running could enqueue the same URL twice.

## v0.3.2

No more stray console windows while Getta runs.

## Bug fixes

- **Console flash.** Every engine check spawned a console process on Windows, so a terminal window flashed on startup, on every Settings or engine check, and when switching pages. All child processes now spawn without a console window.

## v0.3.1

No more console window behind the app.

## Bug fixes

- **Stray terminal on Windows.** Release builds now declare the Windows GUI subsystem, so no console window opens behind Getta and closing it can no longer end the app.

## v0.3.0

Editorial redesign and a serious feature pass.

## New features

- **Editorial redesign.** New sidebar with numbered sections, warm bone-paper palette with cobalt accent, Fraunces display type, manifest-style download list.
- **Library.** Completed downloads show up as a grid; play videos, audio and images inline, or reveal them in the file manager.
- **Command palette.** Ctrl/Cmd+K opens a fuzzy launcher: jump to any page, or run actions like pause all, resume all, clear finished, and download from clipboard.
- **Tools.** A local workbench: Video to MP3, Video to GIF, Compress video with H.264, powered by the FFmpeg you already have.
- **Playlist picker.** Playlists analyze with a checkbox list; download everything or just the entries you want, at the selected format.
- **Onboarding.** A first-run wizard sets language and theme, installs the download engine, and picks the download folder.
- **Real-time progress.** Speed sparklines, live ETA, transfer phases and an aggregate strip over the queue.
- **Global hotkey.** Ctrl/Cmd+Shift+D fetches from your clipboard from anywhere.
- **Deep links.** getta:// links open Getta and queue the URL.
- **Autostart.** Optional start with the system in Settings.

## Improvements

- Queue view is now sectioned (In transit, Queued, Delivered, Failed) with per-section counts.
- Stats line on Home: files saved and total bytes fetched.
- Fewer, calmer toasts at the bottom center.

## v0.2.0

Desktop feels like a native app. Mobile is no longer planned.

## New features

- **System tray.** Getta adds a tray icon: left click toggles the window, right click opens a menu with Show and Quit. Labels follow the configured language.

## Improvements

- **Desktop only.** The mobile plan is dropped; documentation now states Windows, macOS and Linux only.
- **Version on About.** The About screen reads the version from the package manifest, not a hardcoded string.
- **Leaner repository.** Unused Android and iOS icon sets from early generation are removed.

## v0.1.1

Fix for a blank home screen on first launch.

## Bug fixes

- **Blank window.** The app crashed on startup because the language module used Svelte 5 runes in a plain TypeScript file, which the compiler never processed. The module is now compiled as a Svelte module and the app renders normally.

## v0.1.0

First public release. Paste a link, get the file.

## New features

- **Getup.** One input handles single links, pasted batches and direct file URLs, with platform detection, a media preview, quality picker and audio/video-only modes.
- **Download engine.** Local downloads run through yt-dlp and FFmpeg. Getta can install and verify both itself on Windows and macOS; on Linux it uses your distro package.
- **Queue with resume.** A persistent queue with pause, resume, cancel, retry with backoff, live speed, ETA and per-site platform labels. Concurrency, connection count and retries are configurable.
- **History and reveal.** Finished downloads are listed, can be re-downloaded, revealed in the file manager or deleted.
- **Bilingual UI.** English and Bahasa Indonesia everywhere, plus light, dark and system themes.
- **No account, no telemetry.** Getta does not phone home. Downloads go straight from the site to your disk through the engine.
