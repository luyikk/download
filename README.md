# durl

High-performance HTTP downloader with concurrent range requests.

This repository contains:

- `durl`: CLI binary
- `download-lib`: async Rust library
- `libdurl`: C ABI wrapper for native integration

## Features

- Multi-task range download (when server supports `Range`)
- Fallback streaming download when `Content-Length` is missing
- Works with URLs that do not expose a filename in path
- Supports custom output filename via CLI option
- Progress and speed reporting

## Install

Install CLI from crates.io:

```bash
cargo install durl
```

Build from source:

```bash
cargo build --release
```

## CLI Usage

```bash
durl -u <URL> [OPTIONS]
```

Options:

- `-u, --url <URL>`: download URL
- `-s, --save-path <PATH>`: output directory or output file path (default: `./`)
- `-t, --tasks <N>`: concurrent task count (default: `15`)
- `-n, --name <FILE_NAME>`: custom output filename (used when `save-path` is a directory)

## Examples

```bash
# Save to current directory (auto filename)
durl -u "https://example.com/file.zip"

# Save to a specific directory
durl -u "https://example.com/file.zip" -s "D:/downloads"

# Save to an exact file path
durl -u "https://example.com/file.zip" -s "D:/downloads/my.zip"

# URL without filename, force output name
durl -u "https://example.com/download?id=123" -s "D:/downloads" -n "package.zip"

# Increase concurrency
durl -u "https://example.com/large.iso" -t 50
```

## Library Usage (`download-lib`)

Add dependency:

```toml
[dependencies]
download-lib = "0.2.5"
tokio = { version = "1", features = ["full"] }
```

Minimal example:

```rust
use download_lib::DownloadFile;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
	let _download = DownloadFile::start_download(
		"https://example.com/download?id=123",
		PathBuf::from("./"),
		15,
		1024 * 1024,
		Some("result.bin".to_string()),
	)
	.await
	.unwrap();
}
```

## Publish Notes

If you use local path dependencies while developing, keep a version requirement for publish.

Example:

```toml
download-lib = { path = "download-lib", version = "0.2.5" }
```

Without `version`, `cargo publish` will fail manifest verification.

