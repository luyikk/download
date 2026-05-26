# durl
High-performance HTTP downloader with concurrent range requests.
高性能多线程 HTTP 下载器，支持并发分段下载。

This repository contains:
本仓库包含：
- `durl` — CLI binary / 命令行工具
- `download-lib` — async Rust library / 异步 Rust 库
- `libdurl` — C ABI wrapper for native integration / C ABI 封装，用于原生集成
- `durl-gui` — cross-platform GUI download manager (eframe/egui) / 跨平台 GUI 下载管理器

---
## Features / 功能特性
| Feature | 功能 |
|---|---|
| Multi-task range download (when server supports `Range`) | 多线程分段下载（服务端支持 `Range` 时） |
| Fallback streaming download when `Content-Length` is missing | 无 `Content-Length` 时自动切换流式下载 |
| Filename detection from `Content-Disposition` or URL query params | 从 `Content-Disposition` 或 URL 参数自动识别文件名 |
| URL-encoded / RFC 5987 filename decoding | 自动解码 URL 编码文件名（如 `%E6%89%8B%E6%9F%84` → `手柄`） |
| Custom output filename via `-n` | 通过 `-n` 自定义输出文件名 |
| Cookie injection via `-c` (JSON format) | 通过 `-c` 注入 Cookie（JSON 格式） |
| indicatif progress bar with speed and ETA | indicatif 进度条，显示速度和剩余时间 |
| Exponential backoff retry (up to 10 retries) | 指数退避重试，最多 10 次 |
| GUI with task list, progress, SHA256, i18n | GUI 带任务列表、进度、SHA256 校验、多语言 |

---
## Install / 安装

Install CLI from crates.io:
从 crates.io 安装命令行工具：
```bash
cargo install durl
```
Build from source:
从源码编译：
```bash
git clone https://github.com/luyikk/download
cd download
cargo build --release
```
Build GUI:
编译 GUI：
```bash
cd durl-gui
cargo build --release
```

---
## CLI Usage / 命令行用法
```
durl -u <URL> [OPTIONS]
```
### Options / 参数
| Flag | Default | Description | 说明 |
|------|---------|-------------|------|
| `-u, --url` | required | Download URL | 下载链接 |
| `-s, --save-path` | `./` | Output directory or full file path | 保存目录或完整路径 |
| `-t, --tasks` | `15` | Concurrent task count | 并发任务数 |
| `-n, --name` | — | Custom output filename | 自定义输出文件名 |
| `-c, --cookies` | — | Cookies in JSON format | JSON 格式的 Cookie |

---
## Examples / 使用示例
```bash
# Save to current directory (auto filename)
# 保存到当前目录（自动识别文件名）
durl -u "https://example.com/file.zip"

# Save to a specific directory
# 保存到指定目录
durl -u "https://example.com/file.zip" -s "D:/downloads"

# Save with a custom filename
# 自定义文件名
durl -u "https://example.com/download?id=123" -s "D:/downloads" -n "package.zip"

# Pass cookies (JSON object)
# 传入 Cookie（JSON 对象格式）
durl -u "https://example.com/private.zip" -c '{"session":"abc123","token":"xyz"}'

# Pass cookies (JSON array)
# 传入 Cookie（JSON 数组格式）
durl -u "https://example.com/private.zip" -c '[{"name":"session","value":"abc123"}]'

# Increase concurrency
# 提高并发数
durl -u "https://example.com/large.iso" -t 50

# Baidu PCS encoded URL with URL-encoded filename
# 百度网盘带编码文件名的下载地址
durl -u "https://xafj-cm11.baidupcs.com/file/xxx?fin=PS4+slim%E6%89%8B%E6%9F%84.zip&..."
# -> saves as: PS4 slim手柄.zip
```

---
## GUI (durl-gui)
A desktop GUI download manager built with [eframe](https://github.com/emilk/egui/tree/master/crates/eframe).
基于 [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) 构建的桌面 GUI 下载管理器。

**Features / 功能：**
- Task list with status, progress bar, speed, ETA, elapsed time, file type, SHA256
  任务列表：状态、进度条、速度、剩余时间、已用时间、文件类型、SHA256
- New download dialog with URL, save path, filename, concurrency, cookie fields
  新建下载对话框：URL、保存路径、文件名、并发数、Cookie
- Pause / Resume / Delete / Re-download via toolbar and right-click menu
  工具栏和右键菜单：暂停/恢复/删除/重新下载
- Copy URL / Copy SHA256 from right-click menu
  右键菜单复制 URL / 复制 SHA256
- Shared log panel for all tasks
  所有任务共享的日志面板
- i18n: built-in zh-CN and en-US, user-extensible via TOML files in app-config dir
  多语言：内置中文/英文，可在配置目录自定义 TOML 语言文件
- Config stored in OS app-config directory (Windows: `%APPDATA%/durl-gui/`)
  配置存储在系统配置目录
- CJK font auto-detection on Windows, macOS, Linux
  自动检测 Windows/macOS/Linux 系统 CJK 字体

**Linux CJK font / Linux 中文字体：**
```bash
sudo apt install fonts-noto-cjk        # Ubuntu/Debian
sudo pacman -S noto-fonts-cjk          # Arch Linux
```

---
## Cookie Format / Cookie 格式
Two JSON formats are supported:
支持两种 JSON 格式：

**Object format / 对象格式：**
```json
{"session": "abc123", "token": "xyz789"}
```
**Array format / 数组格式：**
```json
[
  {"name": "session", "value": "abc123"},
  {"name": "token",   "value": "xyz789"}
]
```

---
## Library Usage / 库使用方式 (`download-lib`)
Add dependency:
添加依赖：
```toml
[dependencies]
download-lib = "0.2.7"
tokio = { version = "1", features = ["full"] }
```
### Signature / 函数签名
```rust
pub async fn start_download<U: IntoUrl>(
    url: U,
    save_path: PathBuf,
    task_count: u64,   // concurrent tasks / 并发任务数
    block: u64,        // block size in bytes / 每块大小（字节）
    custom_filename: Option<String>,
    cookies: Option<String>,         // JSON cookies or None
) -> Result<DownloadFile>
```
### Minimal example / 最简示例
```rust
use download_lib::DownloadFile;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let download = DownloadFile::start_download(
        "https://example.com/download?id=123",
        PathBuf::from("./"),
        15,
        1024 * 1024,
        None,
        None,
    )
    .await
    .unwrap();

    // Wait for completion / 等待完成
    while !download.get_status().is_finish() {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    println!("Saved to: {}", download.get_real_file_path());
}
```
### With cookies / 带 Cookie 示例
```rust
DownloadFile::start_download(
    "https://example.com/private/file.zip",
    PathBuf::from("D:/downloads"),
    10,
    1024 * 1024,
    Some("my_file.zip".to_string()),
    Some(r#"{"session":"abc123"}"#.to_string()),
)
.await?;
```

---
## Publish Notes / 发布注意事项
When publishing `durl` to crates.io, the path dependency must include a version:
发布到 crates.io 时，路径依赖必须同时指定版本号：
```toml
# Correct / 正确
download-lib = { path = "download-lib", version = "0.2.7" }
# Wrong — cargo publish will fail / 错误 — cargo publish 会报错
download-lib = { path = "download-lib" }
```
Publish order:
发布顺序：
1. `cargo publish` in `download-lib/`
2. Wait for crates.io index to update / 等待 crates.io 索引更新
3. `cargo publish` in root / 根目录执行 `cargo publish`

---
## C/C++ Integration / C/C++ 集成
See [`libdurl/README.md`](libdurl/README.md) for the complete C/C++ calling manual.
完整的 C/C++ 调用手册请参阅 [`libdurl/README.md`](libdurl/README.md)。

---
## License
MIT OR Apache-2.0