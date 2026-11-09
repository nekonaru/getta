# Getta

Getta is a free, open source app that fetches media and files from across the
web into your own folders. Windows, macOS, Linux only. No account,
no telemetry, no terminal.

Getta is an original implementation. Do not paste code, assets or text from
other projects into this repository. Every feature is written from scratch.

## Project

| Variable | Value |
|----------|-------|
| `PROJECT_NAME` | Getta |
| `REPO` | nekonaru/getta |
| `AUTHOR_NAME` | Nicolas Dwi Dharma |
| `AUTHOR_GITHUB` | nekonaru |
| `PROJECT_KIND` | app |
| `STACK` | Tauri 2 (Rust), SvelteKit (Svelte 5, TypeScript) |
| `PLATFORMS` | Windows, macOS, Linux |
| `MIN_OS` | Windows 10, macOS 12, Ubuntu 22.04+ |
| `DEFAULT_BRANCH` | main |
| `LICENSE` | MIT |
| `UI_LANGUAGES` | English, Bahasa Indonesia |
| `VERIFICATION` | ci-only |

## Layout

- `src/` - SvelteKit frontend (static adapter, no server)
- `src-tauri/` - Rust backend (Tauri 2)
- `static/` - static assets (icons, web manifest)
- `assets/logo/` - Getta logo source files and usage notes

## Commands

| Task | Command |
|------|---------|
| Frontend dev server | `npm run dev` |
| Frontend build | `npm run build` |
| Type check + lint UI | `npm run check` |
| Unit tests (frontend) | `npm test` |
| Rust compile check (CI) | `cargo check --manifest-path src-tauri/Cargo.toml` |
| App build | `npx tauri build` |

## Rules for contributors (and agents)

- English for code, comments and commit messages. Bahasa Indonesia only in
  `README.id.md`, `CHANGELOG.id.md` and the UI locale files.
- Keep `README.md` and `README.id.md` in sync in the same commit.
- `CHANGELOG.md` and `CHANGELOG.id.md` follow Keep a Changelog ordering with
  newest on top.
- Do not commit local working files (security reports and so on). They are
  listed in `.git/info/exclude` locally by the maintainer.
- One logical change per commit. Run `npm run check` and `npm test` first.
- Native builds are verified in CI. If you change Rust code, expect CI to be
  the source of truth; do not claim it works until CI is green.

