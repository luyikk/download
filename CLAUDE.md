# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workspace Structure

Cargo workspace with resolver = "2", five crates:

| Crate | Role | Published |
|---|---|---|
| `download-lib` | Core async download library (reqwest + tokio) | Yes (0.2.9) |
| `durl` | CLI binary (`structopt` + `indicatif` progress bar) | Yes (0.2.9) |
| `libdurl` | C ABI staticlib wrapping `download-lib` via tokio runtime | No |
| `durl-gui` | Desktop GUI download manager (eframe/egui, ntex server) | No |
| `durl-gui-iced` | New/empty crate (scaffolding, no code yet) | No |

Release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`.

## Build & Test

```bash
# Build everything
cargo build

# Build individual crates
cargo build -p download-lib
cargo build -p durl
cargo build -p durl-gui

# Run tests (only download-lib and libdurl have tests)
cargo test --package download-lib
cargo test -p libdurl

# Lint (CI-enforced)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

# Release
cargo build --release
```

The GUI crate (`durl-gui`) requires system libraries on Linux: `libgtk-3-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev`.

## Core Architecture

### `download-lib` — the library layer

Public API entry point: `DownloadFile::start_download(url, save_path, task_count, block, custom_filename, cookies) -> Result<DownloadFile>` (6 parameters).

**Download flow:**
1. `build_client(cookies)` — creates `reqwest::Client` with Cookie in default headers. Accepts JSON object, JSON array, or plain `name=value` cookie format.
2. `get_size_and_filename()` — sends GET, resolves size from `Content-Length`, filename from `Content-Disposition` (RFC 5987 `filename*` preferred) > query params (`fn`/`fin`/`filename`/`file_name`) > URL path segment > timestamp fallback.
3. **Known size branch** (`start_known_size`): divides into `task_count` equal ranges, spawns one `ReqwestFile` per range. Single-task optimization when `task_count == 1` reuses the initial GET response.
4. **Unknown size branch** (`start_streaming`): sequential append mode, supports resume via `Range: bytes=N-` on retry.

**Key internals:**
- `DownloadInner` — shared state via atomics (`AtomicU64`, `AtomicBool`) plus `OnceCell<DownloadError>`. Safe to inspect from any thread.
- `ReqwestFile` — retries up to 10 times with exponential backoff (300ms base, 5s cap), 10s chunk timeout, 15s request timeout.
- `FileSave` — actor pattern via `aqueue::Actor`. Writes to `<name>.dd` temp file, renames on `finish()`. Deletes stale `.dd` on recreate.
- Speed ticker: runs every 1s, moves `byte_sec_total` → `byte_sec` via atomic swap.

### `durl` — CLI

Single-file `main.rs`. Parses args via `structopt` (`-u url`, `-s save-path`, `-t tasks`, `-n name`, `-c cookies`). Renders an `indicatif` progress bar; adaptive template for known vs unknown size.

### `libdurl` — C ABI

Staticlib. Creates a `tokio::Runtime`, manages downloads in a `slab::Slab<Arc<DownloadItem>>`. Exposes 14 `extern "C"` functions (`durl_create`, `durl_release`, `durl_start*` variants, `durl_suspend`/`restart`, `durl_get_state`, etc.). Error codes map to i32 (1–7). C header at `libdurl/publish_lib/libdurl.h`.

### `durl-gui` — Desktop GUI

Built on `eframe` (egui, wgpu renderer). Windows subsystem, no console.

**Startup:** panic hook → load `UserConfig` (TOML) → init `GuiLogger` (channel-based, non-blocking) → start browser server → `eframe::run_native`.

**`DUrlApp`** (app.rs, ~1800 lines):
- Owns a 2-worker `tokio::Runtime` for spawning downloads
- Task list persisted as JSON to OS app-config dir (`durl-gui-tasks.json`)
- `update()` runs every 200ms: polls download progress, SHA256 results, browser extension channel, drains log buffer
- UI panels: toolbar, sidebar (filter: All/Downloading/Completed), central task table, bottom log panel
- Dialogs: new download, settings, browser extension install guide
- Actions: Pause/Resume/Delete/Redownload/OpenFile/OpenDir via toolbar buttons and right-click context menu

**Browser extension integration:**
- `browser_server.rs`: ntex HTTP server on `127.0.0.1:19283` — `GET /ping` (liveness), `POST /download` (accepts `{url, cookies, filename?}`)
- Extension files (`extension/`) are `include_bytes!`-embedded and extracted to `%APPDATA%/durl-gui/extension/` at runtime
- Extension intercepts `chrome.downloads.onCreated`, cancels browser download, POSTs to durl-gui, or falls back to browser download if durl-gui isn't running

**i18n:** 6 built-in languages (zh-CN, en-US, ja-JP, ru-RU, fr-FR, de-DE) embedded via `include_str!`. On-disk TOML files auto-regenerated at startup. Users can add custom languages by placing `*.toml` files in the lang directory.

**CJK fonts:** Auto-detected from common OS-specific paths at startup. No embedded font — falls back to egui defaults.

## Key Invariants

1. `task_count >= 1`: `max(min(task_count, size/block), 1)`
2. ReqwestFile retries: max 10, delay = `min(300 * (attempt+1), 5000)` ms
3. `is_finish` set true exactly once, after `save_file.finish()` returns
4. Filenames always go through `sanitize_filename()` (strips `<>:"|?*` and control chars, trims trailing dots/spaces)
5. Cookie in `default_headers` means all requests carry it (initial GET + every retry)
6. `launch_browser()` never passes `chrome://` or `edge://` as CLI args (browsers block them)
7. First recorded error wins via `OnceCell::try_set_error`

## Change Propagation

When modifying `start_download` signature: update call sites in `durl/src/main.rs`, `libdurl/src/lib.rs`, `durl-gui/src/app.rs` (start_download + Resume + Redownload handlers).

When bumping `download-lib` version: also bump the pinned version in `durl/Cargo.toml` and `libdurl/Cargo.toml`.

When adding C ABI exports: mirror in `libdurl/publish_lib/libdurl.h`.

When adding i18n keys: add to ALL six `durl-gui/lang/*.toml` files.

Publish order: `download-lib` first, wait for crates.io index update, then `durl`.

A more detailed maintainer reference exists at `.github/skills/download-maintainer/SKILL.md`.
