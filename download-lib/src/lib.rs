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
    /// start download now
    #[inline]
    pub async fn start_download<U: IntoUrl>(
        url: U,
        mut save_path: PathBuf,
        task_count: u64,
        block: u64,
    ) -> Result<Self> {
        let url = url.into_url()?;
        if save_path.is_dir() {
            let file_name = url
                .path_segments()
                .ok_or_else(|| DownloadError::NotFileName(url.clone()))?
                .rev()
                .next()
                .ok_or_else(|| DownloadError::NotFileName(url.clone()))?;
            save_path.push(file_name);
        }

        let (size, response) = Self::get_size(&url).await?;
        let task_count = { max(min(task_count, size / block), 1) };

        let file = Self {
            task_count,
            save_file: Arc::new(FileSave::create(save_path, size)?),
            inner_status: Arc::new(DownloadInner {
                size,
                url,
                is_start: Default::default(),
                is_finish: Default::default(),
                down_size: Default::default(),
                byte_sec_total: Default::default(),
                byte_sec: Default::default(),
                error: OnceCell::default(),
            }),
        };
        file.save_file.init().await?;
        log::trace!("url file:{} init ok size:{}", file.inner_status.url, size);
        if file.size() > 0 {
            let size = file.size();
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
                        let join: JoinHandle<Result<()>> = tokio::spawn(async move {
                            let end = start + down_size - 1;

                            log::trace!(
                                "task:{} start:{} down size:{} end:{} init",
                                i,
                                start,
                                down_size,
                                end
                            );

                            ReqwestFile::new(save_file, inner_status, start, end)
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
                        .store(inner_status.size, Ordering::Release);
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

                    match ReqwestFile::new(save_file.clone(), inner_status.clone(), 0, size - 1)
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
                        .store(inner_status.size, Ordering::Release);
                    inner_status.is_finish.store(true, Ordering::Release);
                });
            }
        } else {
            file.save_file.finish().await?;
            file.inner_status.is_finish.store(true, Ordering::Release);
        }

        Ok(file)
    }

    /// get url file size
    #[inline]
    async fn get_size(url: &Url) -> Result<(u64, Response)> {
        let response = reqwest::Client::new().get(url.as_str()).send().await?;
        if response.status() == StatusCode::OK {
            Ok((
                Self::parse_content_length(response.headers())
                    .ok_or_else(|| DownloadError::NotGetFileSize(url.clone()))?,
                response,
            ))
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
        self.inner_status.size
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

/// download status
pub struct DownloadInner {
    url: Url,
    size: u64,
    down_size: AtomicU64,
    is_start: AtomicBool,
    is_finish: AtomicBool,
    error: OnceCell<DownloadError>,
    byte_sec: AtomicU64,
    byte_sec_total: AtomicU64,
}

impl DownloadInner {
    /// get url
    #[inline]
    pub fn url(&self) -> &str {
        self.url.as_str()
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
        let current =
            self.down_size.load(Ordering::Acquire) as f64 / self.size.max(1) as f64 * 100.0;
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
