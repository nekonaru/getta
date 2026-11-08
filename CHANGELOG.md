# Changelog

All notable changes to Getta are documented here and in `CHANGELOG.id.md`.
Newest version on top.

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
