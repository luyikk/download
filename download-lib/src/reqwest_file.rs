use super::error::{DownloadError, Result};
use super::file_save::{FileSave, IFileSave};
use super::DownloadInner;
use crate::StatusCode;
use aqueue::Actor;
use futures_util::StreamExt;
use reqwest::Response;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

/// http download file
pub(crate) struct ReqwestFile {
    save_file: Arc<Actor<FileSave>>,
    inner_status: Arc<DownloadInner>,
    client: reqwest::Client,
    start: u64,
    end: u64,
    current: u64,
    #[allow(dead_code)]
    streaming: bool,
}

impl ReqwestFile {
    pub fn new(
        save_file: Arc<Actor<FileSave>>,
        inner_status: Arc<DownloadInner>,
        client: reqwest::Client,
        start: u64,
        end: u64,
    ) -> Self {
        Self {
            save_file,
            inner_status,
            client,
            start,
            end,
            current: start,
            streaming: false,
        }
    }

    /// Create a new streaming downloader (for unknown file size)
    pub fn new_streaming(
        save_file: Arc<Actor<FileSave>>,
        inner_status: Arc<DownloadInner>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            save_file,
            inner_status,
            client,
            start: 0,
            end: 0,
            current: 0,
            streaming: true,
        }
    }

    #[inline]
    pub async fn run(&mut self) -> Result<()> {
        while !self.inner_status.is_finish() && self.current < self.end {
            if !self.inner_status.is_start.load(Ordering::Acquire) {
                sleep(Duration::from_secs(1)).await
            } else {
                're: for i in (0..10).rev() {
                    let request_data = {
                        self.client
                            .get(self.inner_status.url.as_str())
                            .header(
                                reqwest::header::RANGE,
                                format!("bytes={}-{}", self.current, self.end),
                            )
                            .send()
                    };

                    match timeout(Duration::from_secs(15), request_data).await {
                        Ok(Ok(response)) => {
                            if response.status() == StatusCode::OK
                                || response.status() == StatusCode::PARTIAL_CONTENT
                            {
                                log::trace!(
                                    "start download url block:{} start:{} end:{} status:{:?}",
                                    self.inner_status.url,
                                    self.current,
                                    self.end,
                                    response.headers().get(reqwest::header::CONTENT_RANGE)
                                );
                                if self.read_stream(response).await? {
                                    break 're;
                                }
                            } else if i > 0 {
                                log::error!(
                                    "download url:{}  status error:{} retry:{i}",
                                    self.inner_status.url,
                                    response.status()
                                );
                            } else {
                                return Err(DownloadError::HttpStatusError(
                                    response.status().to_string(),
                                ));
                            }
                        }
                        Ok(Err(err)) => {
                            if i > 0 {
                                log::error!(
                                    "download url:{} error:{err} retry:{i}",
                                    self.inner_status.url
                                );
                            } else {
                                return Err(DownloadError::ReqwestError { source: err });
                            }
                        }
                        Err(_) => {
                            log::warn!("get url:{} response time out", self.inner_status.url);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub async fn run_once(&mut self, response: Response) -> Result<()> {
        if !self.read_stream(response).await? {
            self.run().await
        } else {
            Ok(())
        }
    }

    /// Streaming download for unknown file size - writes sequentially without range requests
    #[inline]
    pub async fn run_streaming(&mut self, response: Response) -> Result<()> {
        if !self.read_stream_append(response).await? {
            // If stream was interrupted, retry with a new request (no range)
            for i in (0..10).rev() {
                let request_data = self
                    .client
                    .get(self.inner_status.url.as_str())
                    .header(reqwest::header::RANGE, format!("bytes={}-", self.current))
                    .send();

                match timeout(Duration::from_secs(30), request_data).await {
                    Ok(Ok(response)) => {
                        if response.status() == StatusCode::OK
                            || response.status() == StatusCode::PARTIAL_CONTENT
                        {
                            if self.read_stream_append(response).await? {
                                return Ok(());
                            }
                        } else if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
                            // Server doesn't support range, we need to re-download from scratch
                            // but if we already have data, just return what we have
                            log::warn!("Server doesn't support range requests for resume, finishing with current data");
                            return Ok(());
                        } else if i > 0 {
                            log::error!(
                                "streaming download url:{} status error:{} retry:{i}",
                                self.inner_status.url,
                                response.status()
                            );
                        } else {
                            return Err(DownloadError::HttpStatusError(
                                response.status().to_string(),
                            ));
                        }
                    }
                    Ok(Err(err)) => {
                        if i > 0 {
                            log::error!(
                                "streaming download url:{} error:{err} retry:{i}",
                                self.inner_status.url
                            );
                        } else {
                            return Err(DownloadError::ReqwestError { source: err });
                        }
                    }
                    Err(_) => {
                        log::warn!(
                            "streaming download url:{} response time out",
                            self.inner_status.url
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[inline]
    async fn read_stream(&mut self, response: Response) -> Result<bool> {
        let mut stream = response.bytes_stream();
        let is_finish = loop {
            match timeout(Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(buf))) => {
                    self.save_file
                        .write_all_by_offset(&buf, self.current)
                        .await?;
                    let len = buf.len() as u64;
                    self.current += len;
                    self.inner_status.add_down_size(len);
                    if !self.inner_status.is_start() {
                        log::debug!("is suspend");
                        break false;
                    }
                }
                Ok(Some(Err(err))) => {
                    log::error!(
                        "download url:{} buff is error:{}",
                        self.inner_status.url,
                        err
                    );
                    break false;
                }
                Ok(None) => {
                    log::trace!(
                        "download url:{} block:{}-{} response close",
                        self.inner_status.url,
                        self.start,
                        self.end
                    );
                    break true;
                }
                Err(_) => {
                    log::warn!("download url:{} time out", self.inner_status.url);
                    break false;
                }
            }
        };
        Ok(is_finish)
    }

    /// Read stream and append to file (for streaming/unknown size mode)
    #[inline]
    async fn read_stream_append(&mut self, response: Response) -> Result<bool> {
        let mut stream = response.bytes_stream();
        let is_finish = loop {
            match timeout(Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(buf))) => {
                    self.save_file.write_all(&buf).await?;
                    let len = buf.len() as u64;
                    self.current += len;
                    self.inner_status.add_down_size(len);
                    if !self.inner_status.is_start() {
                        log::debug!("is suspend");
                        break false;
                    }
                }
                Ok(Some(Err(err))) => {
                    log::error!(
                        "streaming download url:{} buff is error:{}",
                        self.inner_status.url,
                        err
                    );
                    break false;
                }
                Ok(None) => {
                    log::trace!(
                        "streaming download url:{} response close, downloaded {} bytes",
                        self.inner_status.url,
                        self.current
                    );
                    break true;
                }
                Err(_) => {
                    log::warn!("streaming download url:{} time out", self.inner_status.url);
                    break false;
                }
            }
        };
        Ok(is_finish)
    }
}
