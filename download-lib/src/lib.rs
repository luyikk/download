#![feature(backtrace)]
mod error;
mod file_save;
mod reqwest_file;

use aqueue::Actor;
use error::DownloadError;
use error::Result;
use file_save::FileSave;
use file_save::IFileSave;
use reqwest::{IntoUrl, Url};
use reqwest_file::ReqwestFile;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Down file handler
pub struct DownloadFile {
    task_count: u64,
    save_file: Arc<Actor<FileSave>>,
    inner_status: Arc<DownloadInner>,
}

/// download status
pub struct DownloadInner {
    url: Url,
    size: u64,
    down_size: AtomicU64,
    is_start: AtomicBool,
    is_finish: AtomicBool,
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

    #[inline]
    fn add_down_size(&self, len: u64) {
        self.down_size.fetch_add(len, Ordering::Release);
        self.byte_sec_total.fetch_add(len, Ordering::Release);
    }
}

impl DownloadFile {
    #[inline]
    pub async fn start_download<U: IntoUrl>(
        url: U,
        mut save_path: PathBuf,
        task_count: u64,
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

        let size = Self::get_size(&url).await?;
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
            }),
        };
        file.save_file.init().await?;
        log::trace!("url file:{} init ok size:{}", file.inner_status.url, size);
        if file.size() > 0 {
            let size = file.size();
            file.inner_status.is_start.store(true, Ordering::Release);
            let connect_count = file.computer_connect_count();
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
                    let size = if i == connect_count - 1 {
                        block_size + end_add_size
                    } else {
                        block_size
                    };
                    let start = i * block_size;

                    let save_file = save_file.clone();
                    let inner_status = inner_status.clone();
                    let join: JoinHandle<Result<()>> = tokio::spawn(async move {
                        log::trace!(
                            "task:{} start:{} size:{} end:{} init",
                            i,
                            start,
                            size,
                            start + size
                        );

                        ReqwestFile::new(save_file, inner_status, start, start + size)
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
                        Ok(r) => {
                            if let Err(err) = r {
                                log::error!("http download error:{:?}", err);
                            }
                        }
                        Err(err) => {
                            log::error!("join error:{:?}", err);
                        }
                    }
                }
                if let Err(err) = save_file.finish().await {
                    log::error!("save file finish error:{:?}", err);
                }
                inner_status.is_finish.store(true, Ordering::Release);
            });
        } else {
            file.save_file.finish().await?;
            file.inner_status.is_finish.store(true, Ordering::Release);
        }

        Ok(file)
    }

    /// get url file size
    #[inline]
    async fn get_size(url: &Url) -> Result<u64> {
        let response = reqwest::Client::new().get(url.as_str()).send().await?;
        Self::parse_content_length(response.headers())
            .ok_or_else(|| DownloadError::NotGetFileSize(url.clone()))
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
    fn computer_connect_count(&self) -> u64 {
        let size = self.size();
        if size > 0 {
            if size < 4096 {
                1
            } else {
                let count = size / self.task_count;
                if count < 4096 {
                    size / 4096
                } else {
                    self.task_count
                }
            }
        } else {
            0
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
        self.inner_status.size
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
