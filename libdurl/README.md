# libdurl — C/C++ Calling Manual / C/C++ 调用手册
`libdurl` is a C ABI static library that wraps `download-lib` for use from C/C++, Unity, Unreal Engine, or any language with C FFI.
`libdurl` 是 `download-lib` 的 C ABI 静态库封装，可在 C/C++、Unity、Unreal Engine 或任何支持 C FFI 的语言中使用。
---
## Link Libraries / 链接库
### Windows (MSVC) — using rustls TLS
```
Bcrypt.lib
ws2_32.lib
Ntdll.lib
libdurl.lib
```
### Windows (MSVC) — using openssl TLS
```
Crypt32.lib
ws2_32.lib
Bcrypt.lib
Userenv.lib
Ntdll.lib
Secur32.lib
Ncrypt.lib
libdurl.lib
```
### Android cross-compile / Android 交叉编译
```bash
export TARGET_AR=~/.NDK/arm/bin/arm-linux-androideabi-ar
export TARGET_CC=~/.NDK/arm/bin/arm-linux-androideabi-clang
cargo build --target armv7-linux-androideabi --release
```
---
## API Reference / API 参考
Include the header:
引入头文件：
```cpp
#include "libdurl.h"
```
### Lifecycle / 生命周期
| Function | Description | 说明 |
|----------|-------------|------|
| `durl_create(thread_count)` | Create runtime, returns `DownloadHandler*` | 创建运行时，返回句柄 |
| `durl_release(handler)` | Free all resources | 释放所有资源 |
| `durl_clean(handler, key)` | Remove a completed download entry | 删除已完成的下载记录 |
### Start Download / 开始下载
| Function | Signature | 说明 |
|----------|-----------|------|
| `durl_start` | `(handler, url, path, task, block) -> u64` | Basic start / 基础下载 |
| `durl_start_file_name` | `(handler, url, path, file_name, task, block) -> u64` | Custom filename / 自定义文件名 |
| `durl_start_cookies` | `(handler, url, path, task, block, cookies) -> u64` | With cookies / 带 Cookie |
| `durl_start_file_name_cookies` | `(handler, url, path, file_name, task, block, cookies) -> u64` | Custom filename + cookies |
All functions return a `key` (download ID). `key == 0` means the start failed.
所有函数返回 `key`（下载 ID）。`key == 0` 表示启动失败。
**Parameters / 参数：**
- `url` — HTTP URL, null-terminated C string / 以 `\0` 结尾的 C 字符串
- `path` — Output directory path / 输出目录路径
- `file_name` — Override output filename (or `nullptr` to auto-detect) / 覆盖输出文件名（`nullptr` 则自动识别）
- `task` — Concurrent range task count (e.g. `15`) / 并发分段任务数
- `block` — Block size in bytes (e.g. `1024*1024`) / 每块字节数
- `cookies` — JSON cookie string or `nullptr` / JSON Cookie 字符串，可为 `nullptr`
### Status / 状态查询
| Function | Signature | Description | 说明 |
|----------|-----------|-------------|------|
| `durl_is_downloading` | `(handler, key) -> bool` | True while running | 运行中返回 true |
| `durl_is_downloading_finish` | `(handler, key) -> bool` | True when finished | 完成时返回 true |
| `durl_get_state` | `(handler, key, *size, *down_size, *err_code) -> u32` | Fill progress fields; returns error msg len | 填充进度字段；返回错误信息长度 |
| `durl_get_error_str` | `(handler, key, buf)` | Copy error string to `buf` | 将错误信息复制到 `buf` |
### Control / 控制
| Function | Description | 说明 |
|----------|-------------|------|
| `durl_suspend(handler, key)` | Pause download | 暂停下载 |
| `durl_restart(handler, key)` | Resume download | 恢复下载 |
### File Paths / 文件路径
| Function | Description | 说明 |
|----------|-------------|------|
| `durl_get_save_file_path(handler, key, buf) -> u32` | Temp `.dd` path; returns string length | 临时 `.dd` 路径；返回字符串长度 |
| `durl_get_real_file_path(handler, key, buf) -> u32` | Final output path; returns string length | 最终输出路径；返回字符串长度 |
### Cookie Format / Cookie 格式
**JSON object / JSON 对象：**
```json
{"session": "abc123", "token": "xyz"}
```
**JSON array / JSON 数组：**
```json
[{"name": "session", "value": "abc123"}]
```
---
## Complete C++ Example / 完整 C++ 示例
```cpp
#include <iostream>
#include <string>
#include <Windows.h>
#include "libdurl.h"
// Returns true when download is done (success or error)
// 当下载完成时（成功或失败）返回 true
bool check(DownloadHandler* runtime, uint64_t key) {
    if (!durl_is_downloading(runtime, key)) {
        std::cout << key << " download not started" << std::endl;
        return false;
    }
    uint64_t size = 0;
    uint64_t down_size = 0;
    int32_t error_code = 0;
    uint32_t len = durl_get_state(runtime, key, &size, &down_size, &error_code);
    if (error_code != 0) {
        std::string msg(len, '\0');
        durl_get_error_str(runtime, key, msg.data());
        std::cout << "Error: " << msg << std::endl;
        durl_clean(runtime, key);
        return true;
    }
    std::cout << key << " progress: " << down_size << "/" << size << " bytes" << std::endl;
    if (size > 0 && size == down_size) {
        std::cout << key << " download finished!" << std::endl;
        durl_clean(runtime, key);
        return true;
    }
    return false;
}
int main() {
    // Create runtime with 2 worker threads / 创建 2 工作线程的运行时
    auto* runtime = durl_create(2);
    // Basic download / 基础下载
    auto key1 = durl_start(
        runtime,
        "https://example.com/file1.zip",
        "D:/downloads/",
        15,
        1024 * 1024
    );
    // Download with cookies / 带 Cookie 下载
    auto key2 = durl_start_cookies(
        runtime,
        "https://example.com/private/file2.zip",
        "D:/downloads/",
        10,
        1024 * 1024,
        R"({"session":"abc123","token":"xyz789"})"
    );
    // Download with custom filename / 自定义文件名下载
    auto key3 = durl_start_file_name(
        runtime,
        "https://example.com/download?id=999",
        "D:/downloads/",
        "my_output.zip",
        15,
        1024 * 1024
    );
    bool done1 = false, done2 = false, done3 = false;
    while (!done1 || !done2 || !done3) {
        if (!done1) done1 = check(runtime, key1);
        if (!done2) done2 = check(runtime, key2);
        if (!done3) done3 = check(runtime, key3);
        Sleep(200);
    }
    durl_release(runtime);
    return 0;
}
```
---
## Getting File Path / 获取文件路径
```cpp
// Get final output path after download completes
// 下载完成后获取最终输出路径
uint32_t len = durl_get_real_file_path(runtime, key, nullptr);
std::string path(len, '\0');
durl_get_real_file_path(runtime, key, path.data());
std::cout << "Saved to: " << path << std::endl;
```
---
## Suspend and Resume / 暂停与恢复
```cpp
durl_suspend(runtime, key);   // pause  暂停
Sleep(5000);
durl_restart(runtime, key);   // resume 恢复
```
---
## Error Codes / 错误码
| i32 | Meaning | 含义 |
|-----|---------|------|
| 0 | No error | 无错误 |
| 1 | HTTP/network failure | HTTP/网络错误 |
| 2 | File I/O failure | 文件 I/O 错误 |
| 3 | (legacy) | 旧版 |
| 4 | Write after closed | 文件已关闭后写入 |
| 5 | (legacy) | 旧版 |
| 6 | Non-2xx HTTP status | HTTP 非 2xx 状态码 |
| 7 | Tokio task panic | Tokio 任务 panic |
---
## Prebuilt Binaries / 预编译文件
Prebuilt static libraries are in `publish_lib/`:
预编译静态库位于 `publish_lib/` 目录：
| Directory | Target |
|-----------|--------|
| `win64/release/` | Windows x86_64 (MSVC) |
| `linux64/release/` | Linux x86_64 |
| `aarch64/release/` | Linux ARM64 / Android ARM64 |
| `armv7a/release/` | Android ARMv7 |
Header file: `publish_lib/libdurl.h`
头文件：`publish_lib/libdurl.h`
---
## License
MIT OR Apache-2.0