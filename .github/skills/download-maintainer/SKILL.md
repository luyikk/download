---
name: download-maintainer
description: >
  Maintain the durl/download-lib/libdurl repository.
  Use this skill when modifying source code, adding features, updating APIs,
  bumping versions, or preparing crates for publishing.
---
# Download Maintainer Skill
Read this document completely before making any change to this repository.
## Repository Map
```
D:\rustprojects\download\
├── Cargo.toml                    # durl binary crate  (version 0.2.7)
├── README.md                     # root doc (keep in sync with CLI)
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
└── libdurl/
    ├── Cargo.toml                # C ABI staticlib    (version 0.1.0)
    ├── src/lib.rs                # extern "C" exports
    └── publish_lib/libdurl.h     # C header (mirror lib.rs exports)
```
## Current Versions
| Crate | Version |
|---|---------|
| durl | 0.2.7   |
| download-lib | 0.2.7   |
| libdurl | 0.1.0   |
Deps:
- Cargo.toml         -> download-lib = { path = "download-lib", version = "0.2.7" }
- libdurl/Cargo.toml -> download-lib = { path = "../download-lib", version = "0.2.7" }
When bumping download-lib bump BOTH pins above.
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
- src/main.rs        : start_download(opt.url, opt.save_path, opt.tasks, 1024*1024, opt.name, opt.cookies)
- libdurl/src/lib.rs : spawn_download() -> DownloadFile::start_download(url, save_path, task, block, file_name, cookies)
## CLI Opt Struct
| Field     | Flag             | Default |
|-----------|------------------|---------|
| url       | -u / --url       | required |
| save_path | -s / --save-path | "./" |
| tasks     | -t / --tasks     | 15 |
| name      | -n / --name      | None |
| cookies   | -c / --cookies   | None |
## C ABI Exports
All durl_start* functions call spawn_download(handler, url, path, task, block, file_name, cookies).
- durl_create / durl_release / durl_clean
- durl_start(handler, url, path, task, block)                          -- cookies=None
- durl_start_file_name(handler, url, path, file_name, task, block)     -- cookies=None
- durl_start_cookies(handler, url, path, task, block, cookies)         -- NEW
- durl_start_file_name_cookies(handler, url, path, file_name, task, block, cookies) -- NEW
- durl_is_downloading / durl_is_downloading_finish
- durl_suspend / durl_restart
- durl_get_state / durl_get_error_str
- durl_get_save_file_path / durl_get_real_file_path
Rule: every new extern "C" fn must also be declared in libdurl/publish_lib/libdurl.h
## Architecture
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
## Error Codes
| Variant         | i32 | Meaning              |
|-----------------|-----|----------------------|
| ReqwestError    |  1  | HTTP/network failure |
| IoError         |  2  | File I/O failure     |
| NotGetFileSize  |  3  | (legacy)             |
| SaveFileFinish  |  4  | Write after closed   |
| NotFileName     |  5  | (legacy)             |
| HttpStatusError |  6  | Non-2xx status       |
| JoinInError     |  7  | Tokio join panic     |
## Change Propagation Rules
Adding/removing a start_download parameter:
  1. Edit download-lib/src/lib.rs signature
  2. Update src/main.rs call
  3. Update libdurl/src/lib.rs spawn_download()
  4. If CLI: add to Opt struct and wire through main()
  5. Update README.md params table + examples
  6. Update download-lib/README.md signature block
Adding a new C ABI export:
  1. Add #[no_mangle] extern "C" fn to libdurl/src/lib.rs
  2. Add declaration to libdurl/publish_lib/libdurl.h
  3. Update libdurl/README.md API table
Bumping download-lib version:
  1. download-lib/Cargo.toml -> new version
  2. Root Cargo.toml -> update pin
  3. libdurl/Cargo.toml -> update pin
  4. cargo build in root and libdurl/
  5. Update version strings in all READMEs and this SKILL.md
## Checklists
Before any change:
  - Read download-lib/src/lib.rs, confirm start_download param count (currently 6)
  - Find call sites: Select-String -Path src\main.rs,libdurl\src\lib.rs -Pattern "start_download"
After any change:
  - cargo build at D:\rustprojects\download\
  - cargo build inside libdurl\
  - cargo test --package download-lib -> 0 failed
  - libdurl.h declares every extern "C" fn in lib.rs
  - README.md examples match actual CLI flags
Before publishing:
  - Both path deps have version = "X.Y.Z"
  - cargo publish --dry-run in download-lib/ passes
  - Publish download-lib first, wait for crates.io index
  - cargo publish --dry-run in root passes
  - Publish durl
## Commands
```powershell
cd D:\rustprojects\download; cargo build
cd D:\rustprojects\download\libdurl; cargo build
cd D:\rustprojects\download; cargo test --package download-lib
Select-String -Path "src\main.rs","libdurl\src\lib.rs" -Pattern "start_download"
cd D:\rustprojects\download\download-lib; cargo publish --dry-run
cd D:\rustprojects\download; cargo publish --dry-run
```
## Troubleshooting
"all dependencies must have a version requirement when publishing"
  Fix: download-lib = { path = "download-lib", version = "0.2.6" }
"this function takes N arguments but M arguments were supplied"
  Fix: align start_download call sites to current 6-param signature
C export missing from libdurl.h
  Fix: add declaration manually to libdurl/publish_lib/libdurl.h
.dd file left on disk
  Expected: deleted automatically on next download start, no action needed
## Invariants (never break)
1. task_count >= 1: max(min(task_count, size/block), 1)
2. Ranges cover size exactly: debug_assert_eq!(block_size*connect_count+end_add_size, size)
3. is_finish set true exactly once, after save_file.finish() returns
4. try_set_error is idempotent -- first error wins
5. Cookie in default_headers -> all requests carry it (initial GET + every retry)
6. All filenames pass sanitize_filename() before disk write