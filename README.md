<div align="center">
  <img src="https://raw.githubusercontent.com/nekonaru/getta/main/assets/logo/getta-app-icon.svg" alt="Getta" width="120" />
  <h1>Getta: Every link, delivered.</h1>
  <p><b>Paste a link. Getta saves the video, audio, image or file straight to your folder.<br/>Free, open source, no account, no terminal.</b></p>
  <a href="https://github.com/nekonaru/getta/releases/latest"><img src="https://img.shields.io/github/v/release/nekonaru/getta?style=flat-square&color=00C896" alt="Latest release" /></a>
  <a href="https://github.com/nekonaru/getta/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/nekonaru/getta/ci.yml?branch=main&style=flat-square&label=CI&color=00C896" alt="CI status" /></a>
  <img src="https://img.shields.io/badge/Tauri%202%20%7C%20Rust%20%7C%20Svelte%205-00C896?style=flat-square" alt="Stack: Tauri 2, Rust, Svelte 5" />
  <img src="https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-desktop-00C896?style=flat-square" alt="Platforms" />
  <a href="https://github.com/nekonaru/getta/blob/main/LICENSE"><img src="https://img.shields.io/github/license/nekonaru/getta?style=flat-square" alt="License: MIT" /></a>
</div>

<br />

**English** | [Bahasa Indonesia](README.id.md)

> [!NOTE]
> Getta is an early release. Core downloads work today. Native extractors, torrents, courses, Telegram and the mobile apps are on the roadmap below, not in this build.

---

## Table of Contents

- [About](#about)
- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Project Structure](#project-structure)
- [Development](#development)
- [Testing](#testing)
- [Security](#security)
- [Continuous Integration](#continuous-integration)
- [Roadmap](#roadmap)
- [FAQ](#faq)
- [Author](#author)
- [License](#license)

---

## About

Getta is a free, open source app for fetching media and files from across the web. You paste a link, Getta shows you what it found, and the file lands in your folder. Downloads run locally through [yt-dlp](https://github.com/yt-dlp/yt-dlp) and [FFmpeg](https://ffmpeg.org/). No account, no telemetry about what you fetch, no terminal.

Getta is an original implementation under the MIT license.

---

## Features

### Fetch anything

| Feature | Description |
|---------|-------------|
| **Omnibox** | One input for single links, multiple pasted links and direct file URLs |
| **Platform detection** | Recognizes YouTube, Instagram, TikTok, X, Reddit, Twitch, Vimeo, Bilibili, SoundCloud and more, plus generic web pages and direct files |
| **Media preview** | Title, author, duration, thumbnail and an on-demand quality list before you download |
| **Modes** | Video + audio, audio only (m4a), or video only |
| **Batch pastes** | Many links at once: Getta queues them all |
| **Filename template** | yt-dlp field template with live defaults, e.g. `%(title).200s [%(id)s].%(ext)s` |

### A queue that behaves

| Feature | Description |
|---------|-------------|
| **Pause and resume** | Process-level pause on Windows and Linux/macOS; downloads pick up where they stopped |
| **Retry with backoff** | Automatic retry for rate limits and transient errors with exponential backoff |
| **Concurrency** | Configurable simultaneous downloads, connections per download and retry count |
| **Speed limit** | Cap the download rate globally (MB/s), changeable from the Downloads screen |
| **Live stats** | Speed, ETA, downloaded bytes and percent per item |
| **History** | Finished downloads with size, actions to re-download, reveal or delete |

### Made to be used

| Feature | Description |
|---------|-------------|
| **Bilingual UI** | English and Bahasa Indonesia everywhere |
| **Light, dark and system themes** | Follows the OS, switchable in Settings |
| **No account, no telemetry** | Getta does not collect anything about what you download |
| **Self-updating engine** | Optional verified install of yt-dlp and FFmpeg, with SHA-256 checking |

---

## Requirements

### To use Getta

| Dependency | Minimum Version | Notes |
|------------|-----------------|-------|
| **Windows** | 10 (x64) | WebView2 runtime, preinstalled on Windows 11 |
| **macOS** | 12 (Monterey) | Apple Silicon and Intel |
| **Linux** | Ubuntu 22.04+ / Fedora 37+ | WebKitGTK 4.1 (`libwebkit2gtk-4.1-0`) |

> [!TIP]
> No build tools required for the installers. Linux users with an older distro can try the AppImage instead of the deb or rpm.

### To build from source

| Dependency | Minimum Version | Notes |
|------------|-----------------|-------|
| **Node.js** | 22 LTS or newer | npm included |
| **Rust** | stable (via rustup) | MSVC on Windows, system toolchain on macOS/Linux |
| **Linux headers** | Ubuntu 22.04+ | `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev` |

### Never used a terminal? Start here

<details><summary><b>Open a terminal on your system</b></summary>

1. **Windows.** Press `Win + R`, type `cmd`, press Enter. Or search "Terminal" in the Start menu.
2. **macOS.** Press `Cmd + Space`, type "Terminal", press Enter.
3. **Linux.** Press `Ctrl + Alt + T`, or find "Terminal" in your apps.

</details>

---

## Installation

### Option A: Download (easiest)

1. Go to the [Releases page](https://github.com/nekonaru/getta/releases/latest).
2. Download the file for your system:
   - **Windows.** `getta_x.y.z_x64-setup.exe` (installer) or the `.msi` for IT deployments.
   - **macOS.** `getta_x.y.z_aarch64.dmg` (Apple Silicon) or `getta_x.y.z_x64.dmg` (Intel).
   - **Linux.** `.deb` (Debian/Ubuntu), `.rpm` (Fedora/openSUSE) or `.AppImage` (everything else).
3. Open the file and follow the prompts.

> [!IMPORTANT]
> Getta is not signed with a paid certificate. On Windows, SmartScreen shows a warning: click **More info**, then **Run anyway**. On macOS, if Gatekeeper says the app is damaged, run `xattr -dr com.apple.quarantine /Applications/Getta.app` once after dragging it into Applications.

### Option B: Build from source (for development / to read or change the code)

#### 1. Get the code

<details><summary><b>I have Git installed</b></summary>

```bash
git clone https://github.com/nekonaru/getta.git
cd getta
```

</details>

<details><summary><b>I don't have Git</b></summary>

1. Click **Code** on the repository page, then **Download ZIP**.
2. Extract it and open a terminal in the extracted folder.

</details>

#### 2. Install dependencies and verify

```bash
npm ci
npm run check
npm test
```

#### 3. Build or run

```bash
npx tauri dev      # run the app in development mode (compiles Rust first)
npx tauri build    # produce installers in src-tauri/target/release/bundle
```

---

## Usage

### First launch

1. Open Getta. The home screen asks for a single URL; a banner at the top tells you if the download engine is missing, with a button to install it.
2. Paste a link (try a public video). Getta reads it and shows a preview.
3. Pick a quality and a mode, then press **Download**.
4. Watch it in the Downloads tab.

> [!NOTE]
> On first run, Getta may ask where to save files. Default is your system Downloads folder; change it in Settings, Output.

### Everyday use

- **Paste many links at once.** Getta detects them and queues every one.
- **Audio only.** Switch the mode to "Audio only" before downloading.
- **Pause.** Use the pause button on an item; resume when you want. On Windows, process-level suspend keeps partial files byte-consistent.
- **Limit the rate.** Use the speed limit select on Downloads, or the setting in Settings, Transfer.
- **Login-only sites.** In Settings, Engine, set the browser Getta should read cookies from (e.g. `chrome`, `firefox`).
- **Clear the queue.** "Clear finished" removes completed and failed items from the list.

---

## Configuration

Getta stores its data in one folder:

| Platform | Location |
|----------|----------|
| Windows | `%APPDATA%\getta` |
| macOS | `~/Library/Application Support/getta` |
| Linux | `~/.local/share/getta` |

Inside it: `settings.json` (atomic write), `history.json` (last 200 entries), `bin/` (managed yt-dlp and FFmpeg). Set `GETTA_DATA_DIR` to override the location.

Settings live in **Settings** inside the app: output folder and file name template, concurrency, speed limit, engine paths, cookies and proxy.

---

## Project Structure

```
getta/
├── src/                    # SvelteKit frontend (Svelte 5)
│   ├── routes/             # Home, Downloads, Settings, About
│   ├── lib/                # API bridge, stores, i18n (en/id), formatting
│   └── app.css             # design tokens and base styles
├── src-tauri/              # Rust backend (Tauri 2)
│   ├── src/                # queue, yt-dlp management, settings, history
│   ├── icons/              # application icons (all targets)
│   └── tauri.conf.json
├── static/                 # favicon and web manifest
├── assets/logo/            # Getta logo source files
├── README.md               # this file
└── README.id.md            # Indonesian translation
```

---

## Development

### Adding a user-facing string

1. Add the key to `src/lib/i18n/en.json` and `src/lib/i18n/id.json` in the same commit.
2. Use `t("section.key")` in components. English is the source; Indonesian mirrors it.

### Releasing

1. Bump `version` in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` in the same commit.
2. Update `CHANGELOG.md` and `CHANGELOG.id.md`.
3. Push a `vX.Y.Z` tag. CI builds and attaches installers to the GitHub release.

---

## Testing

```bash
npm test
```

| Check | Requirement |
|-------|-------------|
| `npm run check` | Zero errors, zero warnings (svelte-check, fail-on-warnings) |
| `npm test` | Unit tests pass (vitest) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust unit tests pass (in CI; needs a C toolchain locally) |
| `npm run build` | Vite production build succeeds |

---

## Security

Getta downloads its engines from their official publishers and verifies yt-dlp's SHA-256 against the upstream sums file before installing; it refuses to install when verification fails. Settings and history are written atomically and a corrupt file is quarantined and reset rather than wiped blindly. There is no telemetry and no account; the only network calls go to the sites you download from and to the engine publishers when you install or update. See [`SECURITY.md`](SECURITY.md) for how to report a vulnerability.

---

## Continuous Integration

| Workflow | Purpose |
|----------|---------|
| `ci.yml` | Frontend check, tests and build; `cargo check` and `cargo test` on Linux, Windows and macOS |
| `release.yml` | On `v*` tags: builds installers for Linux, Windows and macOS and publishes them to the GitHub release |

---

## Roadmap

Planned, not shipped: native extractors for the big platforms (faster, cookie-aware), bittorrent support, course downloaders, Telegram integration, a command-line mode, Android and iOS builds. Nothing here is promised until a release tag ships it.

---

## FAQ

<details><summary><b>Does Getta need Python?</b></summary>

No. Getta downloads and runs the official yt-dlp binary. Python is not involved.

</details>

<details><summary><b>Can Getta download DRM-protected videos?</b></summary>

No. Getta does not bypass DRM, paywalls or access controls.

</details>

<details><summary><b>Where do files go?</b></summary>

Your system Downloads folder by default, or whatever you pick in Settings. With "Sort into folders per site" on, files land in per-platform subfolders.

</details>

<details><summary><b>Why does Windows warn about the installer?</b></summary>

Getta is open source and not signed with a paid certificate. That is normal for free desktop apps. See Installation for how to proceed.

</details>

<details><summary><b>Does Getta work on phones?</b></summary>

Not yet. Android and iOS builds are on the roadmap.

</details>

---

## Author

<div align="center">

| |
|:-:|
| [![Nicolas Dwi Dharma](https://github.com/nekonaru.png?size=100)](https://github.com/nekonaru) |
| **[Nicolas Dwi Dharma](https://github.com/nekonaru)** |

</div>

---

## License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for more information.
