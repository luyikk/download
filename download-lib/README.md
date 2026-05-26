# download-lib
Async Rust HTTP download library with multi-task range download, streaming fallback, cookie support, and automatic filename detection.
异步 Rust HTTP 下载库，支持多线程分段下载、流式回退、Cookie 注入和自动识别文件名。
## Add Dependency / 添加依赖
```toml
[dependencies]
download-lib = "0.2.6"
tokio = { version = "1", features = ["full"] }
```
---
## API
### `DownloadFile::start_download`
```rust
pub async fn start_download<U: IntoUrl>(
    url: U,
    save_path: PathBuf,
    task_count: u64,          // concurrent range tasks / 并发分段任务数
    block: u64,               // bytes per block / 每块大小（字节）
    custom_filename: Option<String>,
    cookies: Option<String>,  // JSON cookies or None / JSON Cookie，可为 None
) -> Result<DownloadFile>
```
**Returns / 返回值:** A `DownloadFile` handle. The download runs in background tasks; poll `get_status()` to track progress.
返回 `DownloadFile` 句柄，下载在后台任务运行，通过 `get_status()` 轮询进度。
### `DownloadFile` methods / 方法
| Method | Description | 说明 |
|--------|-------------|------|
| `get_status() -> DownloadStatus` | Get a cloneable status handle | 获取可共享的状态句柄 |
| `get_real_file_path() -> String` | Final output file path | 最终输出文件路径 |
### `DownloadStatus` methods / 方法
| Method | Return | Description | 说明 |
|--------|--------|-------------|------|
| `is_finish() -> bool` | bool | Download finished (success or error) | 下载已完成（成功或失败） |
| `is_error() -> bool` | bool | Download ended with error | 下载出错 |
| `get_size() -> u64` | u64 | Total file size (0 if unknown) | 文件总大小（未知时为 0） |
| `get_down_size() -> u64` | u64 | Bytes downloaded so far | 已下载字节数 |
| `get_byte_sec() -> u64` | u64 | Current speed (bytes/sec) | 当前速度（字节/秒） |
| `get_error() -> Option<String>` | Option<String> | Error message, if any | 错误信息（如有） |
---
## Filename Detection Priority / 文件名识别优先级
1. `Content-Disposition: filename*=` (RFC 5987) — always wins / 最高优先级
2. `Content-Disposition: filename=` — URL-decoded / 自动解码
3. URL query parameter: `filename` / `file_name` / `fn` / `fin`
4. URL path last segment (if it contains `.`)
5. Timestamped fallback: `download_YYYYMMDD_HHMMSS.bin`
All filenames are sanitized (strips `<>:"|?*` and control chars).
所有文件名都会被清理（去除 `<>:"|?*` 等非法字符）。
---
## Cookie Format / Cookie 格式
Pass as the `cookies` parameter (JSON string):
通过 `cookies` 参数传入（JSON 字符串）：
**Object format / 对象格式:**
```json
{"session": "abc123", "token": "xyz789"}
```
**Array format / 数组格式:**
```json
[{"name": "session", "value": "abc123"}, {"name": "token", "value": "xyz789"}]
```
Cookies are injected into the `Cookie` request header for all requests including retries.
Cookie 会注入到所有请求（包括重试请求）的 `Cookie` 请求头中。
---
## Examples / 示例
### Basic download / 基础下载
```rust
use download_lib::DownloadFile;
use std::path::PathBuf;
use std::time::Duration;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let download = DownloadFile::start_download(
        "https://example.com/file.zip",
        PathBuf::from("./"),
        15,
        1024 * 1024,
        None,
        None,
    )
    .await?;
    while !download.get_status().is_finish() {
        let s = download.get_status();
        println!("{}/{} bytes  speed: {}/s",
            s.get_down_size(), s.get_size(), s.get_byte_sec());
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    if !download.get_status().is_error() {
        println!("Saved to: {}", download.get_real_file_path());
    }
    Ok(())
}
```
### Download with cookies and custom filename / 带 Cookie 和自定义文件名
```rust
let download = DownloadFile::start_download(
    "https://example.com/private/large.zip",
    PathBuf::from("D:/downloads"),
    10,
    1024 * 1024,
    Some("output.zip".to_string()),
    Some(r#"{"session":"abc123","user_id":"42"}"#.to_string()),
)
.await?;
```
### Streaming download (no Content-Length) / 流式下载（无 Content-Length）
The library detects automatically — no special code needed.
库会自动检测，无需特殊处理。
```rust
// Same API, streaming is auto-selected when server omits Content-Length
// 相同的 API，服务端不返回 Content-Length 时自动切换为流式下载
let download = DownloadFile::start_download(
    "https://example.com/stream/video.mp4",
    PathBuf::from("./"),
    1,
    1024 * 1024,
    None,
    None,
)
.await?;
```
---
## Download Modes / 下载模式
| Condition | Mode | 模式 |
|-----------|------|------|
| Server returns `Content-Length` | Parallel range download | 并行分段下载 |
| `Content-Length` missing | Streaming download (append) | 流式下载（追加写入） |
**Range mode:** pre-allocates file, splits into N blocks, each fetched by a separate task.
**分段模式：** 预分配文件，分成 N 个块，每个块由独立任务下载。
**Streaming mode:** single task, appends chunks as they arrive.
**流式模式：** 单任务，数据到来时追加写入。
---
## Retry Strategy / 重试策略
- Max retries: **10**  最多重试 10 次
- Delay formula: `min(300 × (attempt + 1), 5000) ms`
  - attempt 0 → 300 ms, attempt 1 → 600 ms, … attempt 9 → 3000 ms (capped at 5000 ms)
- Chunk read timeout: **10 seconds** per chunk  每个数据块 10 秒超时
---
## File Handling / 文件处理
- While downloading, the file is written to `<output>.dd` (temporary)
- On successful completion, renamed to the final filename
- Any stale `.dd` file from a previous interrupted download is deleted on start
下载过程中，文件会写入 `<output>.dd`（临时文件）；
下载成功后重命名为最终文件名；
启动时会自动删除上次中断留下的 `.dd` 文件。
---
## Error Codes / 错误码
| i32 | Variant | Meaning | 含义 |
|-----|---------|---------|------|
| 1 | `ReqwestError` | HTTP/network failure | HTTP/网络错误 |
| 2 | `IoError` | File I/O failure | 文件 I/O 错误 |
| 3 | `NotGetFileSize` | (legacy) | （旧版） |
| 4 | `SaveFileFinish` | Write after closed | 文件已关闭后写入 |
| 5 | `NotFileName` | (legacy) | （旧版） |
| 6 | `HttpStatusError` | Non-2xx HTTP status | HTTP 非 2xx 状态码 |
| 7 | `JoinInError` | Tokio task panic | Tokio 任务 panic |
---
## License
MIT OR Apache-2.0