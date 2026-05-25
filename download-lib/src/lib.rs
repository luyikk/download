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
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Down file handler
pub struct DownloadFile {
    task_count: u64,
    save_file: Arc<Actor<FileSave>>,
    inner_status: Arc<DownloadInner>,
}

impl DownloadFile {
    /// Build a shared reqwest client with common settings
    fn build_client() -> reqwest::Client {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }

    /// start download now
    /// if `custom_filename` is Some, use it as the saved filename (when save_path is a directory)
    #[inline]
    pub async fn start_download<U: IntoUrl>(
        url: U,
        mut save_path: PathBuf,
        task_count: u64,
        block: u64,
        custom_filename: Option<String>,
    ) -> Result<Self> {
        let url = url.into_url()?;
        let client = Self::build_client();
        let (size, file_name, response) = Self::get_size_and_filename(&client, &url).await?;
        if save_path.is_dir() {
            let final_name = if let Some(name) = custom_filename {
                name
            } else if let Some(name) = file_name {
                name
            } else {
                Self::extract_filename_from_url(&url)
            };
            save_path.push(final_name);
        }

        // If size is unknown, use streaming mode (single task)
        if let Some(size) = size {
            // Known size mode - original logic
            let task_count = { max(min(task_count, size / block), 1) };

            let file = Self {
                task_count,
                save_file: Arc::new(FileSave::create(save_path, Some(size))?),
                inner_status: Arc::new(DownloadInner {
                    size: AtomicU64::new(size),
                    url,
                    is_start: Default::default(),
                    is_finish: Default::default(),
                    down_size: Default::default(),
                    byte_sec_total: Default::default(),
                    byte_sec: Default::default(),
                    error: OnceCell::default(),
                    size_known: true,
                }),
            };
            file.save_file.init().await?;
            log::trace!("url file:{} init ok size:{}", file.inner_status.url, size);
            if size > 0 {
                file.inner_status.is_start.store(true, Ordering::Release);
                let connect_count = file.task_count;

                if connect_count > 1 {
                    drop(response);
                    let block_size = size / connect_count;
                    let end_add_size = size % block_size;
                    assert_eq!(block_size * connect_count + end_add_size, size);
                    log::trace!(
                        "computer task count:{}  block size:{} end add size:{}",
                        connect_count,
                        block_size,
                        end_add_size
                    );
                    let save_file = file.save_file.clone();
                    let inner_status = file.inner_status.clone();
                    let client = client.clone();
                    tokio::spawn(async move {
                        let mut join_vec = Vec::with_capacity(connect_count as usize);
                        for i in 0..connect_count {
                            let down_size = if i == connect_count - 1 {
                                block_size + end_add_size
                            } else {
                                block_size
                            };
                            let start = i * block_size;

                            let save_file = save_file.clone();
                            let inner_status = inner_status.clone();
                            let client = client.clone();
                            let join: JoinHandle<Result<()>> = tokio::spawn(async move {
                                let end = start + down_size - 1;

                                log::trace!(
                                    "task:{} start:{} down size:{} end:{} init",
                                    i,
                                    start,
                                    down_size,
                                    end
                                );

                                ReqwestFile::new(save_file, inner_status, client, start, end)
                                    .run()
                                    .await?;
                                log::trace!("task:{} finish", i);
                                Ok(())
                            });
                            join_vec.push(join);
                        }

                        let inner_status_sec = inner_status.clone();
                        tokio::spawn(async move {
                            while !inner_status_sec.is_finish() {
                                inner_status_sec.byte_sec.store(
                                    inner_status_sec.byte_sec_total.swap(0, Ordering::Release),
                                    Ordering::Release,
                                );
                                sleep(Duration::from_secs(1)).await
                            }
                        });

                        for task in join_vec {
                            match task.await {
                                Ok(Err(err)) => {
                                    log::error!("http download error:{:?}", err);
                                    if !inner_status.error.initialized() {
                                        if let Err(err) = inner_status.error.set(err) {
                                            log::error!("set error fail:{}", err)
                                        }
                                    }
                                }
                                Err(err) => {
                                    log::error!("join error:{:?}", err);
                                    if !inner_status.error.initialized() {
                                        if let Err(err) =
                                            inner_status.error.set(DownloadError::JoinInError(err))
                                        {
                                            log::error!("set error fail:{}", err)
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Err(err) = save_file.finish().await {
                            log::error!("save file finish error:{:?}", err);
                            if !inner_status.error.initialized() {
                                if let Err(err) = inner_status.error.set(err) {
                                    log::error!("set error fail:{}", err)
                                }
                            }
                        }
                        inner_status
                            .down_size
                            .store(inner_status.get_size(), Ordering::Release);
                        inner_status.is_finish.store(true, Ordering::Release);
                    });
                } else {
                    let save_file = file.save_file.clone();
                    let inner_status = file.inner_status.clone();

                    tokio::spawn(async move {
                        let inner_status_sec = inner_status.clone();
                        tokio::spawn(async move {
                            while !inner_status_sec.is_finish() {
                                inner_status_sec.byte_sec.store(
                                    inner_status_sec.byte_sec_total.swap(0, Ordering::Release),
                                    Ordering::Release,
                                );
                                sleep(Duration::from_secs(1)).await
                            }
                        });

                        log::trace!(
                            "start once task download url:{} size:{}",
                            inner_status.url,
                            size
                        );

                        match ReqwestFile::new(
                            save_file.clone(),
                            inner_status.clone(),
                            client,
                            0,
                            size - 1,
                        )
                        .run_once(response)
                        .await
                        {
                            Err(err) => {
                                log::error!("http download error:{:?}", err);
                                if !inner_status.error.initialized() {
                                    if let Err(err) = inner_status.error.set(err) {
                                        log::error!("set error fail:{}", err)
                                    }
                                }
                            }
                            _ => {}
                        }

                        if let Err(err) = save_file.finish().await {
                            log::error!("save file finish error:{:?}", err);
                            if !inner_status.error.initialized() {
                                if let Err(err) = inner_status.error.set(err) {
                                    log::error!("set error fail:{}", err)
                                }
                            }
                        }

                        inner_status
                            .down_size
                            .store(inner_status.get_size(), Ordering::Release);
                        inner_status.is_finish.store(true, Ordering::Release);
                    });
                }
            } else {
                file.save_file.finish().await?;
                file.inner_status.is_finish.store(true, Ordering::Release);
            }

            Ok(file)
        } else {
            // Unknown size - streaming mode
            log::trace!("url file:{} size unknown, using streaming mode", url);
            let file = Self {
                task_count: 1,
                save_file: Arc::new(FileSave::create(save_path, None)?),
                inner_status: Arc::new(DownloadInner {
                    size: AtomicU64::new(0),
                    url,
                    is_start: Default::default(),
                    is_finish: Default::default(),
                    down_size: Default::default(),
                    byte_sec_total: Default::default(),
                    byte_sec: Default::default(),
                    error: OnceCell::default(),
                    size_known: false,
                }),
            };
            file.save_file.init().await?;
            file.inner_status.is_start.store(true, Ordering::Release);

            let save_file = file.save_file.clone();
            let inner_status = file.inner_status.clone();

            tokio::spawn(async move {
                let inner_status_sec = inner_status.clone();
                tokio::spawn(async move {
                    while !inner_status_sec.is_finish() {
                        inner_status_sec.byte_sec.store(
                            inner_status_sec.byte_sec_total.swap(0, Ordering::Release),
                            Ordering::Release,
                        );
                        sleep(Duration::from_secs(1)).await
                    }
                });

                log::trace!("start streaming download url:{}", inner_status.url);

                match ReqwestFile::new_streaming(save_file.clone(), inner_status.clone(), client)
                    .run_streaming(response)
                    .await
                {
                    Err(err) => {
                        log::error!("http download error:{:?}", err);
                        if !inner_status.error.initialized() {
                            if let Err(err) = inner_status.error.set(err) {
                                log::error!("set error fail:{}", err)
                            }
                        }
                    }
                    _ => {}
                }

                if let Err(err) = save_file.finish().await {
                    log::error!("save file finish error:{:?}", err);
                    if !inner_status.error.initialized() {
                        if let Err(err) = inner_status.error.set(err) {
                            log::error!("set error fail:{}", err)
                        }
                    }
                }

                // Set final size
                inner_status
                    .size
                    .store(inner_status.get_down_size(), Ordering::Release);
                inner_status.is_finish.store(true, Ordering::Release);
            });

            Ok(file)
        }
    }

    /// Extract filename from URL path, with fallback to generated name
    fn extract_filename_from_url(url: &Url) -> String {
        // Try to extract from URL path
        if let Some(segments) = url.path_segments() {
            if let Some(last) = segments.rev().next() {
                let decoded = urlencoding_decode(last);
                if !decoded.is_empty() && decoded != "/" && decoded.contains('.') {
                    return decoded;
                }
            }
        }

        // Try to guess extension from URL path
        let path = url.path();
        let ext = if path.contains('.') {
            path.rsplit('.').next().unwrap_or("bin")
        } else {
            "bin"
        };

        // Generate filename with timestamp
        let now = chrono::Local::now();
        format!("download_{}.{}", now.format("%Y%m%d_%H%M%S"), ext)
    }

    /// get url file size and file name
    #[inline]
    async fn get_size_and_filename(
        client: &reqwest::Client,
        url: &Url,
    ) -> Result<(Option<u64>, Option<String>, Response)> {
        let response = client.get(url.as_str()).send().await?;

        let status = response.status();
        if status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT {
            let filename = Self::parse_content_filename(response.headers());
            let size = Self::parse_content_length(response.headers());

            // Also try to get filename from final URL (after redirects)
            let final_filename = if filename.is_none() {
                Self::parse_content_filename_from_url(response.url())
            } else {
                filename
            };

            log::trace!("url:{} size:{:?} filename:{:?}", url, size, final_filename);

            Ok((size, final_filename, response))
        } else {
            Err(DownloadError::HttpStatusError(
                response.status().to_string(),
            ))
        }
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

        disposition.trim().split(';').find_map(|content| {
            let content = content.trim();
            // Handle both filename and filename*
            if content.starts_with("filename*=") {
                // RFC 5987: filename*=UTF-8''encoded_name
                let value = content.split('\'').last()?;
                Some(urlencoding_decode(value))
            } else if content.starts_with("filename=") {
                let value = content.split('=').nth(1)?;
                // Remove surrounding quotes if present
                let value = value.trim_matches('"').trim_matches('\'');
                Some(value.to_string())
            } else {
                None
            }
        })
    }

    /// Try to extract a meaningful filename from the final URL (after redirects)
    #[inline]
    fn parse_content_filename_from_url(url: &Url) -> Option<String> {
        let segments = url.path_segments()?;
        let last = segments.rev().next()?;
        let decoded = urlencoding_decode(last);
        if !decoded.is_empty() && decoded != "/" && decoded.contains('.') {
            Some(decoded)
        } else {
            None
        }
    }

    /// get url
    #[inline]
    pub fn url(&self) -> &str {
        self.inner_status.url()
    }

    /// get status arc
    #[inline]
    pub fn get_status(&self) -> Arc<DownloadInner> {
        self.inner_status.clone()
    }

    /// file size
    #[inline]
    pub fn size(&self) -> u64 {
        self.inner_status.get_size()
    }

    /// get down size
    #[inline]
    pub fn get_down_size(&self) -> u64 {
        self.inner_status.get_down_size()
    }

    /// is start
    #[inline]
    pub fn is_start(&self) -> bool {
        self.inner_status.is_start()
    }

    /// is finish
    #[inline]
    pub fn is_finish(&self) -> bool {
        self.inner_status.is_finish()
    }

    /// is error
    #[inline]
    pub fn is_error(&self) -> bool {
        self.inner_status.is_error()
    }

    /// get error
    #[inline]
    pub fn get_error(&self) -> Option<&DownloadError> {
        self.inner_status.get_error()
    }

    /// get save file real path
    #[inline]
    pub fn get_real_file_path(&self) -> String {
        self.save_file.get_real_file_path()
    }

    /// get save file real path
    #[inline]
    pub fn get_save_file_path(&self) -> String {
        self.save_file.get_save_file_path()
    }

    /// suspend download
    #[inline]
    pub fn suspend(&self) {
        self.inner_status.is_start.store(false, Ordering::Release);
    }

    /// restart download
    #[inline]
    pub fn restart(&self) {
        self.inner_status.is_start.store(true, Ordering::Release);
    }
}

/// Simple percent-decoding for URL components
fn urlencoding_decode(s: &str) -> String {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

/// download status
pub struct DownloadInner {
    url: Url,
    size: AtomicU64,
    down_size: AtomicU64,
    is_start: AtomicBool,
    is_finish: AtomicBool,
    error: OnceCell<DownloadError>,
    byte_sec: AtomicU64,
    byte_sec_total: AtomicU64,
    size_known: bool,
}

impl DownloadInner {
    /// get url
    #[inline]
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// get size
    #[inline]
    pub fn get_size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    /// is start
    #[inline]
    pub fn is_start(&self) -> bool {
        self.is_start.load(Ordering::Acquire)
    }

    /// is finish
    #[inline]
    pub fn is_finish(&self) -> bool {
        self.is_finish.load(Ordering::Acquire)
    }

    /// is error
    #[inline]
    pub fn is_error(&self) -> bool {
        self.error.initialized()
    }

    /// get error
    #[inline]
    pub fn get_error(&self) -> Option<&DownloadError> {
        self.error.get()
    }

    /// get complete percent
    #[inline]
    pub fn get_percent_complete(&self) -> f64 {
        let size = self.get_size();
        if !self.size_known && !self.is_finish() {
            // Unknown size, can't compute percentage
            return 0.0;
        }
        if size == 0 {
            return if self.is_finish() { 100.0 } else { 0.0 };
        }
        let current = self.down_size.load(Ordering::Acquire) as f64 / size.max(1) as f64 * 100.0;
        (current * 100.0).round() / 100.0
    }

    /// computer bs
    #[inline]
    pub fn get_byte_sec(&self) -> u64 {
        self.byte_sec.load(Ordering::Acquire)
    }

    /// get size
    #[inline]
    pub fn get_down_size(&self) -> u64 {
        self.down_size.load(Ordering::Acquire)
    }

    /// add down size
    #[inline]
    fn add_down_size(&self, len: u64) {
        self.down_size.fetch_add(len, Ordering::Release);
        self.byte_sec_total.fetch_add(len, Ordering::Release);
    }
}
