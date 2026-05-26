mod error;
mod file_save;
mod reqwest_file;

use aqueue::Actor;
pub use error::DownloadError;
use error::Result;
use file_save::FileSave;
use file_save::IFileSave;
use reqwest::{IntoUrl, Response, StatusCode, Url};
use reqwest_file::ReqwestFile;
use std::cmp::{max, min};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tokio::time::sleep;

/// Down file handler
pub struct DownloadFile {
    task_count: u64,
    save_file: Arc<Actor<FileSave>>,
    inner_status: Arc<DownloadInner>,
}

impl DownloadFile {
    /// Build a shared reqwest client.
    /// `cookies_json`: optional JSON — object `{"k":"v"}` or array `[{"name":"k","value":"v"}]`.
    fn build_client(cookies_json: Option<&str>) -> reqwest::Client {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(json) = cookies_json {
            if let Some(cookie_str) = Self::parse_cookies_json(json) {
                if let Ok(val) = reqwest::header::HeaderValue::from_str(&cookie_str) {
                    headers.insert(reqwest::header::COOKIE, val);
                }
            }
        }
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }

    /// Parse a JSON cookie string into `name=value; ...` header format.
    fn parse_cookies_json(json: &str) -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(json).ok()?;
        let mut pairs: Vec<String> = Vec::new();
        match &v {
            serde_json::Value::Object(map) => {
                for (k, val) in map {
                    let value = match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    pairs.push(format!("{}={}", k, value));
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let (Some(name), Some(value)) = (
                        item.get("name").and_then(|v| v.as_str()),
                        item.get("value").and_then(|v| v.as_str()),
                    ) {
                        pairs.push(format!("{}={}", name, value));
                    }
                }
            }
            _ => return None,
        }
        if pairs.is_empty() {
            None
        } else {
            Some(pairs.join("; "))
        }
    }

    /// Start a download.
    /// - `custom_filename`: override the saved filename when `save_path` is a directory.
    /// - `cookies`: optional JSON cookies — object `{"k":"v"}` or array `[{"name":"k","value":"v"}]`.
    #[inline]
    pub async fn start_download<U: IntoUrl>(
        url: U,
        mut save_path: PathBuf,
        task_count: u64,
        block: u64,
        custom_filename: Option<String>,
        cookies: Option<String>,
    ) -> Result<Self> {
        let url = url.into_url()?;
        let client = Self::build_client(cookies.as_deref());
        let (size, file_name, response) = Self::get_size_and_filename(&client, &url).await?;

        if save_path.is_dir() {
            let final_name = custom_filename
                .or(file_name)
                .unwrap_or_else(|| Self::extract_filename_from_url(&url));
            save_path.push(final_name);
        }

        match size {
            Some(size) => {
                Self::start_known_size(url, save_path, task_count, block, size, client, response)
                    .await
            }
            None => Self::start_streaming(url, save_path, client, response).await,
        }
    }

    /// Download branch: file size is known — supports multi-task parallel range download.
    async fn start_known_size(
        url: Url,
        save_path: PathBuf,
        task_count: u64,
        block: u64,
        size: u64,
        client: reqwest::Client,
        response: Response,
    ) -> Result<Self> {
        let task_count = max(min(task_count, size / block), 1);
        let file = Self {
            task_count,
            save_file: Arc::new(FileSave::create(save_path, Some(size))?),
            inner_status: Arc::new(DownloadInner::new_known(url, size)),
        };
        file.save_file.init().await?;
        log::debug!("url:{} init ok size:{}", file.inner_status.url, size);

        // Empty file — nothing to download.
        if size == 0 {
            file.save_file.finish().await?;
            file.inner_status.is_finish.store(true, Ordering::Release);
            return Ok(file);
        }

        file.inner_status.is_start.store(true, Ordering::Release);
        let connect_count = file.task_count;
        let save_file = file.save_file.clone();
        let inner_status = file.inner_status.clone();

        if connect_count > 1 {
            // Multi-task: divide file into equal-sized blocks.
            drop(response); // release initial HEAD/GET connection
            let block_size = size / connect_count;
            let end_add_size = size % connect_count;
            debug_assert_eq!(block_size * connect_count + end_add_size, size);
            log::debug!(
                "multi-task count:{} block:{} tail:{}",
                connect_count,
                block_size,
                end_add_size
            );

            tokio::spawn(async move {
                spawn_speed_ticker(inner_status.clone());

                let mut join_vec = Vec::with_capacity(connect_count as usize);

                for i in 0..connect_count {
                    let chunk_size = if i == connect_count - 1 {
                        block_size + end_add_size
                    } else {
                        block_size
                    };
                    let start = i * block_size;
                    let end = start + chunk_size - 1;

                    log::debug!("task:{} range:{}-{}", i, start, end);
                    join_vec.push(tokio::spawn(
                        ReqwestFile::new(
                            save_file.clone(),
                            inner_status.clone(),
                            client.clone(),
                            start,
                            end,
                        )
                        .run(),
                    ));
                }

                for task in join_vec {
                    match task.await {
                        Ok(Err(err)) => {
                            log::error!("task error:{:?}", err);
                            inner_status.try_set_error(err);
                        }
                        Err(err) => {
                            log::error!("join error:{:?}", err);
                            inner_status.try_set_error(DownloadError::JoinInError(err));
                        }
                        _ => {}
                    }
                }

                if let Err(err) = save_file.finish().await {
                    log::error!("finish error:{:?}", err);
                    inner_status.try_set_error(err);
                }
                inner_status
                    .down_size
                    .store(inner_status.get_size(), Ordering::Release);
                inner_status.is_finish.store(true, Ordering::Release);
            });
        } else {
            // Single-task: reuse the already-open response.
            tokio::spawn(async move {
                spawn_speed_ticker(inner_status.clone());
                log::debug!("single-task url:{} size:{}", inner_status.url, size);

                if let Err(err) =
                    ReqwestFile::new(save_file.clone(), inner_status.clone(), client, 0, size - 1)
                        .run_once(response)
                        .await
                {
                    log::error!("task error:{:?}", err);
                    inner_status.try_set_error(err);
                }

                if let Err(err) = save_file.finish().await {
                    log::error!("finish error:{:?}", err);
                    inner_status.try_set_error(err);
                }
                inner_status
                    .down_size
                    .store(inner_status.get_size(), Ordering::Release);
                inner_status.is_finish.store(true, Ordering::Release);
            });
        }

        Ok(file)
    }

    /// Download branch: file size unknown — single sequential stream.
    async fn start_streaming(
        url: Url,
        save_path: PathBuf,
        client: reqwest::Client,
        response: Response,
    ) -> Result<Self> {
        log::debug!("size unknown, streaming mode: {}", url);
        let file = Self {
            task_count: 1,
            save_file: Arc::new(FileSave::create(save_path, None)?),
            inner_status: Arc::new(DownloadInner::new_streaming(url)),
        };
        file.save_file.init().await?;
        file.inner_status.is_start.store(true, Ordering::Release);

        let save_file = file.save_file.clone();
        let inner_status = file.inner_status.clone();

        tokio::spawn(async move {
            spawn_speed_ticker(inner_status.clone());
            log::debug!("streaming url:{}", inner_status.url);

            if let Err(err) =
                ReqwestFile::new_streaming(save_file.clone(), inner_status.clone(), client)
                    .run_streaming(response)
                    .await
            {
                log::error!("streaming error:{:?}", err);
                inner_status.try_set_error(err);
            }

            if let Err(err) = save_file.finish().await {
                log::error!("finish error:{:?}", err);
                inner_status.try_set_error(err);
            }
            // For streaming, total size = bytes actually received.
            inner_status
                .size
                .store(inner_status.get_down_size(), Ordering::Release);
            inner_status.is_finish.store(true, Ordering::Release);
        });

        Ok(file)
    }

    /// Resolve filename from URL path, falling back to a timestamped name.
    fn extract_filename_from_url(url: &Url) -> String {
        if let Some(mut segments) = url.path_segments() {
            if let Some(last) = segments.next_back() {
                let decoded = urlencoding_decode(last);
                if !decoded.is_empty() && decoded != "/" && decoded.contains('.') {
                    return decoded;
                }
            }
        }
        let ext = url
            .path()
            .rsplit('.')
            .next()
            .filter(|e| !e.contains('/'))
            .unwrap_or("bin");
        format!(
            "download_{}.{}",
            chrono::Local::now().format("%Y%m%d_%H%M%S"),
            ext
        )
    }

    /// HEAD/GET the URL to resolve file size and filename from response headers.
    #[inline]
    async fn get_size_and_filename(
        client: &reqwest::Client,
        url: &Url,
    ) -> Result<(Option<u64>, Option<String>, Response)> {
        let response = client.get(url.as_str()).send().await?;
        let status = response.status();
        if status != StatusCode::OK && status != StatusCode::PARTIAL_CONTENT {
            return Err(DownloadError::HttpStatusError(status.to_string()));
        }

        let filename = Self::parse_content_filename(response.headers());
        let size = Self::parse_content_length(response.headers());
        // Priority: Content-Disposition > query params (fn/fin/filename) > URL path segment
        let final_filename = filename
            .or_else(|| Self::parse_content_filename_from_query(response.url()))
            .or_else(|| Self::parse_content_filename_from_url(response.url()));

        log::debug!("url:{} size:{:?} filename:{:?}", url, size, final_filename);
        Ok((size, final_filename, response))
    }

    #[inline]
    fn parse_content_length(headers: &reqwest::header::HeaderMap) -> Option<u64> {
        headers
            .get(reqwest::header::CONTENT_LENGTH)?
            .to_str()
            .ok()?
            .parse::<u64>()
            .ok()
    }

    #[inline]
    fn parse_content_filename(headers: &reqwest::header::HeaderMap) -> Option<String> {
        let disposition = headers
            .get(reqwest::header::CONTENT_DISPOSITION)?
            .to_str()
            .ok()?;

        // Prefer filename* (RFC 5987) over filename=
        let mut plain: Option<String> = None;
        for part in disposition.trim().split(';') {
            let part = part.trim();
            if part.starts_with("filename*=") {
                let value = part.split_once('=')?.1;
                let value = if let Some(pos) = value.rfind("''") {
                    &value[pos + 2..]
                } else {
                    value
                };
                return Some(sanitize_filename(&form_decode(value.trim_matches('"'))));
            } else if part.starts_with("filename=") && plain.is_none() {
                let value = part.split_once('=')?.1;
                let value = value.trim_matches('"').trim_matches('\'');
                plain = Some(sanitize_filename(&form_decode(value)));
            }
        }
        plain
    }

    #[inline]
    fn parse_content_filename_from_url(url: &Url) -> Option<String> {
        let last = url.path_segments()?.next_back()?;
        let decoded = urlencoding_decode(last);
        if !decoded.is_empty() && decoded != "/" && decoded.contains('.') {
            Some(sanitize_filename(&decoded))
        } else {
            None
        }
    }

    /// Extract filename from URL query params (`filename`, `file_name`, `fn`, `fin`).
    /// Uses form-decode so `+` → space and `%xx` → UTF-8.
    #[inline]
    fn parse_content_filename_from_query(url: &Url) -> Option<String> {
        let query = url.query()?;
        for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
            if matches!(k.as_ref(), "filename" | "file_name" | "fn" | "fin") {
                let value = v.trim().to_string();
                if !value.is_empty() {
                    return Some(sanitize_filename(&value));
                }
            }
        }
        None
    }

    // ── Public accessors ──────────────────────────────────────────────────────

    #[inline]
    pub fn url(&self) -> &str {
        self.inner_status.url()
    }

    #[inline]
    pub fn get_status(&self) -> Arc<DownloadInner> {
        self.inner_status.clone()
    }

    /// Total file size (0 when unknown until streaming completes).
    #[inline]
    pub fn size(&self) -> u64 {
        self.inner_status.get_size()
    }

    #[inline]
    pub fn get_down_size(&self) -> u64 {
        self.inner_status.get_down_size()
    }

    #[inline]
    pub fn is_start(&self) -> bool {
        self.inner_status.is_start()
    }

    #[inline]
    pub fn is_finish(&self) -> bool {
        self.inner_status.is_finish()
    }

    #[inline]
    pub fn is_error(&self) -> bool {
        self.inner_status.is_error()
    }

    #[inline]
    pub fn get_error(&self) -> Option<&DownloadError> {
        self.inner_status.get_error()
    }

    /// Final file path (after `.dd` temp file is renamed).
    #[inline]
    pub fn get_real_file_path(&self) -> String {
        self.save_file.get_real_file_path()
    }

    /// Temporary `.dd` file path used during download.
    #[inline]
    pub fn get_save_file_path(&self) -> String {
        self.save_file.get_save_file_path()
    }

    /// Pause all download tasks.
    #[inline]
    pub fn suspend(&self) {
        self.inner_status.is_start.store(false, Ordering::Release);
    }

    /// Resume a paused download.
    #[inline]
    pub fn restart(&self) {
        self.inner_status.is_start.store(true, Ordering::Release);
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

/// Spawn the 1-second speed ticker that moves byte_sec_total → byte_sec.
fn spawn_speed_ticker(inner: Arc<DownloadInner>) {
    tokio::spawn(async move {
        while !inner.is_finish() {
            inner.byte_sec.store(
                inner.byte_sec_total.swap(0, Ordering::Release),
                Ordering::Release,
            );
            sleep(Duration::from_secs(1)).await;
        }
    });
}

/// Percent-decode a URL path component (`%xx` → UTF-8, `+` is literal).
fn urlencoding_decode(s: &str) -> String {
    percent_decode_bytes(s.as_bytes(), false)
}

/// Decode `application/x-www-form-urlencoded`: `+` → space, `%xx` → UTF-8.
fn form_decode(s: &str) -> String {
    percent_decode_bytes(s.as_bytes(), true)
}

fn percent_decode_bytes(bytes: &[u8], plus_as_space: bool) -> String {
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' if plus_as_space => {
                result.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hi) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                    if let Ok(byte) = u8::from_str_radix(hi, 16) {
                        result.push(byte);
                        i += 3;
                        continue;
                    }
                }
                result.push(bytes[i]);
                i += 1;
            }
            b => {
                result.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(result).unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
}

/// Replace characters illegal in Windows/Linux filenames; trim trailing dots/spaces.
fn sanitize_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0'..='\x1f' => '_',
            c => c,
        })
        .collect();
    s.trim_end_matches(['.', ' ']).to_string()
}

// ── DownloadInner ─────────────────────────────────────────────────────────────

/// Shared download progress/state, safe to inspect from any thread.
pub struct DownloadInner {
    url: Url,
    size: AtomicU64,
    down_size: AtomicU64,
    is_start: AtomicBool,
    is_finish: AtomicBool,
    error: OnceCell<DownloadError>,
    byte_sec: AtomicU64,
    byte_sec_total: AtomicU64,
    /// false while streaming (size not yet known).
    size_known: bool,
}

impl DownloadInner {
    fn new_known(url: Url, size: u64) -> Self {
        Self {
            url,
            size: AtomicU64::new(size),
            size_known: true,
            is_start: Default::default(),
            is_finish: Default::default(),
            down_size: Default::default(),
            byte_sec_total: Default::default(),
            byte_sec: Default::default(),
            error: OnceCell::default(),
        }
    }

    fn new_streaming(url: Url) -> Self {
        Self {
            url,
            size: AtomicU64::new(0),
            size_known: false,
            is_start: Default::default(),
            is_finish: Default::default(),
            down_size: Default::default(),
            byte_sec_total: Default::default(),
            byte_sec: Default::default(),
            error: OnceCell::default(),
        }
    }

    /// Store `err` if no error has been recorded yet. Silently drops duplicate errors.
    fn try_set_error(&self, err: DownloadError) {
        if !self.error.initialized() {
            if let Err(e) = self.error.set(err) {
                log::error!("try_set_error: duplicate error ignored: {}", e);
            }
        }
    }

    #[inline]
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    #[inline]
    pub fn get_size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_start(&self) -> bool {
        self.is_start.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_finish(&self) -> bool {
        self.is_finish.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_error(&self) -> bool {
        self.error.initialized()
    }

    #[inline]
    pub fn get_error(&self) -> Option<&DownloadError> {
        self.error.get()
    }

    /// Returns 0.0 while streaming with unknown size; 100.0 only after finish.
    #[inline]
    pub fn get_percent_complete(&self) -> f64 {
        if !self.size_known && !self.is_finish() {
            return 0.0;
        }
        let size = self.get_size();
        if size == 0 {
            return if self.is_finish() { 100.0 } else { 0.0 };
        }
        let pct = self.down_size.load(Ordering::Acquire) as f64 / size as f64 * 100.0;
        (pct * 100.0).round() / 100.0
    }

    /// Instantaneous download speed in bytes/second (updated once per second).
    #[inline]
    pub fn get_byte_sec(&self) -> u64 {
        self.byte_sec.load(Ordering::Acquire)
    }

    #[inline]
    pub fn get_down_size(&self) -> u64 {
        self.down_size.load(Ordering::Acquire)
    }

    #[inline]
    fn add_down_size(&self, len: u64) {
        self.down_size.fetch_add(len, Ordering::Release);
        self.byte_sec_total.fetch_add(len, Ordering::Release);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_decode_handles_plus_as_space() {
        assert_eq!(form_decode("PS4+slim"), "PS4 slim");
    }

    #[test]
    fn form_decode_handles_percent_encoded_utf8() {
        assert_eq!(form_decode("%E6%89%8B%E6%9F%84"), "手柄");
    }

    #[test]
    fn form_decode_baidu_filename() {
        assert_eq!(
            form_decode("PS4+slim%E6%89%8B%E6%9F%84.zip"),
            "PS4 slim手柄.zip"
        );
    }

    #[test]
    fn urlencoding_decode_keeps_plus_literal() {
        assert_eq!(urlencoding_decode("PS4+slim"), "PS4+slim");
    }

    #[test]
    fn urlencoding_decode_handles_percent_encoded_utf8() {
        assert_eq!(urlencoding_decode("%E6%89%8B%E6%9F%84"), "手柄");
    }

    #[test]
    fn sanitize_filename_removes_illegal_chars() {
        assert_eq!(sanitize_filename("a<b>c:d.zip"), "a_b_c_d.zip");
    }

    #[test]
    fn sanitize_filename_trims_trailing_dots() {
        assert_eq!(sanitize_filename("file.zip.."), "file.zip");
    }

    #[test]
    fn parse_content_filename_decodes_header() {
        let mut map = reqwest::header::HeaderMap::new();
        map.insert(
            reqwest::header::CONTENT_DISPOSITION,
            reqwest::header::HeaderValue::from_static(
                "attachment; filename=PS4+slim%E6%89%8B%E6%9F%84.zip",
            ),
        );
        assert_eq!(
            DownloadFile::parse_content_filename(&map).unwrap(),
            "PS4 slim手柄.zip"
        );
    }

    #[test]
    fn parse_content_filename_star_rfc5987() {
        let mut map = reqwest::header::HeaderMap::new();
        map.insert(
            reqwest::header::CONTENT_DISPOSITION,
            reqwest::header::HeaderValue::from_static(
                "attachment; filename*=UTF-8''PS4%20slim%E6%89%8B%E6%9F%84.zip",
            ),
        );
        assert_eq!(
            DownloadFile::parse_content_filename(&map).unwrap(),
            "PS4 slim手柄.zip"
        );
    }

    #[test]
    fn parse_content_filename_star_preferred_over_plain() {
        // When both filename and filename* are present, filename* wins.
        let mut map = reqwest::header::HeaderMap::new();
        map.insert(
            reqwest::header::CONTENT_DISPOSITION,
            reqwest::header::HeaderValue::from_static(
                "attachment; filename=fallback.zip; filename*=UTF-8''preferred.zip",
            ),
        );
        assert_eq!(
            DownloadFile::parse_content_filename(&map).unwrap(),
            "preferred.zip"
        );
    }

    #[test]
    fn parse_content_filename_from_query_decodes() {
        let url: Url = "https://example.com/dl?fn=PS4+slim%E6%89%8B%E6%9F%84.zip"
            .parse()
            .unwrap();
        assert_eq!(
            DownloadFile::parse_content_filename_from_query(&url).unwrap(),
            "PS4 slim手柄.zip"
        );
    }

    #[test]
    fn parse_cookies_json_object() {
        let result =
            DownloadFile::parse_cookies_json(r#"{"session":"abc","token":"xyz"}"#).unwrap();
        assert!(result.contains("session=abc"));
        assert!(result.contains("token=xyz"));
    }

    #[test]
    fn parse_cookies_json_array() {
        let result = DownloadFile::parse_cookies_json(
            r#"[{"name":"session","value":"abc"},{"name":"token","value":"xyz"}]"#,
        )
        .unwrap();
        assert!(result.contains("session=abc"));
        assert!(result.contains("token=xyz"));
    }

    #[test]
    fn parse_cookies_json_empty_returns_none() {
        assert!(DownloadFile::parse_cookies_json("{}").is_none());
        assert!(DownloadFile::parse_cookies_json("[]").is_none());
    }
}
