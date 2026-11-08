# Changelog

All notable changes to Getta are documented here and in `CHANGELOG.id.md`.
Newest version on top.

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

- **Home omnibox.** One input handles single links, pasted batches and direct file URLs, with platform detection, a media preview, quality picker and audio/video-only modes.
- **Download engine.** Local downloads run through yt-dlp and FFmpeg. Getta can install and verify both itself on Windows and macOS; on Linux it uses your distro package.
- **Queue with resume.** A persistent queue with pause, resume, cancel, retry with backoff, live speed, ETA and per-site platform labels. Concurrency, connection count and retries are configurable.
- **History and reveal.** Finished downloads are listed, can be re-downloaded, revealed in the file manager or deleted.
- **Bilingual UI.** English and Bahasa Indonesia everywhere, plus light, dark and system themes.
- **No account, no telemetry.** Getta does not phone home. Downloads go straight from the site to your disk through the engine.
