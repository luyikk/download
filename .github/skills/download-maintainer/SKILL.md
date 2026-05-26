---
name: download-maintainer
description: >
  Maintain the durl/download-lib/libdurl/durl-gui repository.
  Use this skill when modifying source code, adding features, updating APIs,
  bumping versions, or preparing crates for publishing.
---
# Download Maintainer Skill
Read this document completely before making any change to this repository.

## Repository Map
```
D:\rustprojects\download\
├── Cargo.toml                    # durl binary crate  (version 0.2.7)
├── README.md                     # root doc (keep in sync with CLI + GUI)
├── src/
│   └── main.rs                   # CLI: structopt Opt + indicatif progress bar
├── download-lib/
│   ├── Cargo.toml                # library crate      (version 0.2.7)
│   ├── README.md                 # library API manual
│   └── src/
│       ├── lib.rs                # PUBLIC API: DownloadFile, DownloadInner
│       ├── reqwest_file.rs       # HTTP worker: range + streaming
│       ├── file_save.rs          # File I/O actor
│       └── error.rs              # DownloadError enum + i32 code map
├── libdurl/
│   ├── Cargo.toml                # C ABI staticlib    (version 0.1.0)
│   ├── src/lib.rs                # extern "C" exports
│   └── publish_lib/libdurl.h     # C header (mirror lib.rs exports)
└── durl-gui/
    ├── Cargo.toml                # GUI crate           (version 0.1.0)
    ├── build.rs                  # copies lang/ to target, embeds Windows icon
    ├── src/
    │   ├── main.rs               # eframe entry point
    │   ├── app.rs                # DurlApp: task list, toolbar, dialogs
    │   ├── config.rs             # UserConfig (user.toml)
    │   ├── gui_logger.rs         # async mpsc log capture → LogBuffer
    │   ├── i18n.rs               # LangStrings (TOML-based i18n)
    │   └── paths.rs              # platform app-config dir helpers
    ├── lang/
    │   ├── en-US.toml            # embedded via include_str! + written to disk
    │   └── zh-CN.toml
    └── assets/
        ├── letter-d.ico          # window icon (embedded + .exe resource)
        └── gui.png
```

## Current Versions
| Crate        | Version |
|--------------|---------|
| durl         | 0.2.7   |
| download-lib | 0.2.7   |
| libdurl      | 0.1.0   |
| durl-gui     | 0.1.0   |

Deps:
- `Cargo.toml`         → `download-lib = { path = "download-lib", version = "0.2.7" }`
- `libdurl/Cargo.toml` → `download-lib = { path = "../download-lib", version = "0.2.5" }` ⚠️ needs alignment
- `durl-gui/Cargo.toml`→ `download-lib = { path = "../download-lib" }` (no version, not published)

When bumping download-lib bump ALL pins above.

## start_download Signature (6 parameters)
```rust
pub async fn start_download<U: IntoUrl>(
    url: U,
    save_path: PathBuf,
    task_count: u64,
    block: u64,
    custom_filename: Option<String>,
    cookies: Option<String>,
) -> Result<DownloadFile>
```
Call sites to keep in sync:
- `src/main.rs`         : `start_download(opt.url, opt.save_path, opt.tasks, 1024*1024, opt.name, opt.cookies)`
- `libdurl/src/lib.rs`  : `spawn_download()` → `DownloadFile::start_download(url, save_path, task, block, file_name, cookies)`
- `durl-gui/src/app.rs` : `DownloadFile::start_download(url, save_path, task_count, 1024*1024, custom_name, cookies)`

## CLI Opt Struct
| Field     | Flag                 | Default  |
|-----------|----------------------|----------|
| url       | `-u` / `--url`       | required |
| save_path | `-s` / `--save-path` | `"./"`   |
| tasks     | `-t` / `--tasks`     | `15`     |
| name      | `-n` / `--name`      | `None`   |
| cookies   | `-c` / `--cookies`   | `None`   |

## C ABI Exports
All `durl_start*` functions call `spawn_download(handler, url, path, task, block, file_name, cookies)`.

| Function | cookies | file_name |
|----------|---------|-----------|
| `durl_create(thread_count)` | — | — |
| `durl_release(handler)` | — | — |
| `durl_clean(handler, key)` | — | — |
| `durl_start(handler, url, path, task, block)` | None | None |
| `durl_start_file_name(handler, url, path, file_name, task, block)` | None | Some |
| `durl_start_cookies(handler, url, path, task, block, cookies)` | Some | None |
| `durl_start_file_name_cookies(handler, url, path, file_name, task, block, cookies)` | Some | Some |
| `durl_is_downloading(handler, key)` | — | — |
| `durl_is_downloading_finish(handler, key)` | — | — |
| `durl_suspend(handler, key)` | — | — |
| `durl_restart(handler, key)` | — | — |
| `durl_get_state(handler, key)` | — | — |
| `durl_get_error_str(handler, key, msg)` | — | — |
| `durl_get_save_file_path(handler, key, msg)` | — | — |
| `durl_get_real_file_path(handler, key, msg)` | — | — |

Rule: every new `extern "C" fn` must also be declared in `libdurl/publish_lib/libdurl.h`.

## download-lib Architecture
```
start_download()
  build_client(cookies)   -> reqwest::Client with Cookie in default_headers
  get_size_and_filename() -> HTTP GET -> (Option<u64>, Option<String>, Response)
    Filename priority:
      1. Content-Disposition filename*= (RFC 5987) -- always wins
      2. Content-Disposition filename=
      3. URL query: filename | file_name | fn | fin  (form-decode: + -> space)
      4. URL path last segment if contains '.'
      5. timestamped fallback: download_YYYYMMDD_HHMMSS.ext
    All results -> sanitize_filename() -> Windows-safe
  size=Some(n) -> start_known_size()
    task_count = max(min(task_count, size/block), 1)
    count>1 -> pre-alloc file, N x ReqwestFile::run()       [parallel range]
    count=1 -> ReqwestFile::run_once(response)               [single task]
  size=None -> start_streaming()
    ReqwestFile::new_streaming().run_streaming(response)     [append mode]
read_stream_inner(append: bool)
  append=false -> write_all_by_offset  [range, pre-allocated]
  append=true  -> write_all            [streaming, growing]
  chunk timeout: 10s
  retry_delay(attempt) = min(300*(attempt+1), 5000) ms, max 10 retries
FileSave: writes <real>.dd, rename on finish(), deletes stale .dd on create()
spawn_speed_ticker: every 1s byte_sec <- byte_sec_total.swap(0)
```

## durl-gui Architecture
```
main.rs
  init_gui_logger(level) -> LogBuffer   # async mpsc log capture (try_send, non-blocking)
  load_user_config()                    # %APPDATA%/durl-gui/user.toml (Windows)
  eframe::run_native("durl-gui", wgpu renderer, DurlApp::new)

DurlApp (app.rs)
  rt:    tokio::Runtime                 # multi-thread, 2 workers
  tasks: Vec<DownloadTask>             # per-task state (NO logs field)
  logs:  Vec<String>                   # single shared log panel (all tasks, max 5000)
  update_tasks()  -- called each frame: poll sha256_rx, receiver, download status
  drain_lib_logs() -- flush LogBuffer -> self.logs
  render_toolbar / render_sidebar / render_table / render_log_panel
  render_new_dialog / render_settings_dialog

DownloadTask fields (no logs):
  id, url, filename, file_path, save_dir,
  file_size, downloaded, speed, progress,
  status: TaskStatus (Starting|Downloading|Paused|Completed|Error),
  receiver: Option<mpsc::Receiver<Result<DownloadFile>>>,
  download: Option<DownloadFile>,
  sha256: Option<String>,  sha256_rx: Option<mpsc::Receiver<String>>,
  task_count, cookies, elapsed, start_time, error_msg

Persistence (JSON)
  tasks_config_path() -> %APPDATA%/durl-gui/durl-gui-tasks.json   (Windows)
                      -> ~/.config/durl-gui/durl-gui-tasks.json    (Linux)
                      -> ~/Library/Application Support/durl-gui/.. (macOS)
  SHA256 persisted in TaskRecord (serde default = None for old records)

i18n (i18n.rs)
  Built-in: include_str!("../lang/zh-CN.toml") / en-US.toml
  On startup: write to lang_dir() if file missing or content differs from embedded
  available_languages(): scan lang_dir(), read [meta] display_name from each .toml
  Users can add custom languages by placing *.toml in lang_dir()

Font loading (setup_fonts)
  Order: Windows system -> macOS system -> Linux (Noto CJK, WenQuanYi, Arphic, Arch)
  No embedded font — falls back to egui default if none found
  Linux: sudo apt install fonts-noto-cjk  OR  fonts-wqy-microhei

New download dialog
  resizable(true), default_width 600, min 400, max 800
  Rows: URL, Save to (+ Browse), Filename, Threads, Cookie
  Buttons: Cancel | Start (right-aligned via right_to_left layout)
```

## Error Codes
| Variant         | i32 | Meaning              |
|-----------------|-----|----------------------|
| ReqwestError    |  1  | HTTP/network failure |
| IoError         |  2  | File I/O failure     |
| NotGetFileSize  |  3  | (legacy)             |
| SaveFileFinish  |  4  | Write after closed   |
| NotFileName     |  5  | (legacy)             |
| HttpStatusError |  6  | Non-2xx status       |
| JoinInError     |  7  | Tokio task panic     |

## Change Propagation Rules
**Adding/removing a `start_download` parameter:**
  1. Edit `download-lib/src/lib.rs` signature
  2. Update `src/main.rs` call
  3. Update `libdurl/src/lib.rs` `spawn_download()`
  4. Update `durl-gui/src/app.rs` call sites (start_download + handle_action Resume/Redownload)
  5. If CLI: add to `Opt` struct and wire through `main()`
  6. Update `README.md` params table + examples
  7. Update `download-lib/README.md` signature block

**Adding a new C ABI export:**
  1. Add `#[no_mangle] extern "C" fn` to `libdurl/src/lib.rs`
  2. Add declaration to `libdurl/publish_lib/libdurl.h`
  3. Update `libdurl/README.md` API table

**Bumping download-lib version:**
  1. `download-lib/Cargo.toml` → new version
  2. Root `Cargo.toml` → update pin
  3. `libdurl/Cargo.toml` → update pin
  4. `cargo build` in root, `libdurl/`, and `durl-gui/`
  5. Update version strings in all READMEs and this SKILL.md

**Adding a durl-gui i18n key:**
  1. Add key to both `durl-gui/lang/zh-CN.toml` and `en-US.toml`
  2. `include_str!` in `i18n.rs` picks it up at compile time
  3. On next run, on-disk lang files regenerate automatically if content changed

## Checklists
**Before any change:**
  - Read `download-lib/src/lib.rs`, confirm `start_download` param count (currently 6)
  - `Select-String -Path src\main.rs,libdurl\src\lib.rs,durl-gui\src\app.rs -Pattern "start_download"`

**After any change:**
  - `cargo build` at `D:\rustprojects\download\`
  - `cargo build` inside `libdurl\`
  - `cargo build` inside `durl-gui\`
  - `cargo test --package download-lib` → 0 failed
  - `libdurl.h` declares every `extern "C" fn` in `lib.rs`
  - `README.md` examples match actual CLI flags

**Before publishing:**
  - Both path deps have `version = "X.Y.Z"`
  - `cargo publish --dry-run` in `download-lib/` passes
  - Publish `download-lib` first, wait for crates.io index
  - `cargo publish --dry-run` in root passes
  - Publish `durl`

## Commands
```powershell
cd D:\rustprojects\download; cargo build
cd D:\rustprojects\download\libdurl; cargo build
cd D:\rustprojects\download\durl-gui; cargo build
cd D:\rustprojects\download; cargo test --package download-lib
Select-String -Path "src\main.rs","libdurl\src\lib.rs","durl-gui\src\app.rs" -Pattern "start_download"
cd D:\rustprojects\download\download-lib; cargo publish --dry-run
cd D:\rustprojects\download; cargo publish --dry-run
```

## Troubleshooting
**"all dependencies must have a version requirement when publishing"**
  Fix: `download-lib = { path = "download-lib", version = "0.2.7" }`

**"this function takes N arguments but M arguments were supplied"**
  Fix: align all `start_download` call sites to current 6-param signature

**C export missing from libdurl.h**
  Fix: add declaration manually to `libdurl/publish_lib/libdurl.h`

**.dd file left on disk**
  Expected: deleted automatically on next download start, no action needed

**durl-gui Chinese text shows boxes on Linux/macOS**
  Fix (Ubuntu): `sudo apt install fonts-noto-cjk`
  Fix (Arch):   `sudo pacman -S noto-fonts-cjk`
  Fix (macOS):  PingFang.ttc is bundled with the OS; should work out of the box

**durl-gui "Failed to create wgpu adapter" on Windows 10**
  Cause: GPU driver too old or missing Vulkan/DX12 support
  Fix: update graphics driver; app uses wgpu renderer (not OpenGL)

## Invariants (never break)
1. `task_count >= 1`: `max(min(task_count, size/block), 1)`
2. Ranges cover size exactly: `debug_assert_eq!(block_size*connect_count+end_add_size, size)`
3. `is_finish` set true exactly once, after `save_file.finish()` returns
4. `try_set_error` is idempotent — first error wins
5. Cookie in `default_headers` → all requests carry it (initial GET + every retry)
6. All filenames pass `sanitize_filename()` before disk write
7. `DurlApp.logs` is the single shared log buffer; `DownloadTask` has no `logs` field
8. SHA256 computed in `std::thread::spawn` (non-blocking), result via `mpsc::channel`
