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

/// Exponential backoff delay for retry attempt `attempt` (0-based).
/// attempt 0 → 300 ms, 1 → 600 ms, …, 9 → 3 000 ms, capped at 5 s.
#[inline]
fn retry_delay(attempt: u32) -> Duration {
    Duration::from_millis((300 * (attempt as u64 + 1)).min(5_000))
}

/// HTTP range downloader for a single file block.
pub(crate) struct ReqwestFile {
    save_file: Arc<Actor<FileSave>>,
    inner_status: Arc<DownloadInner>,
    client: reqwest::Client,
    start: u64,
    end: u64,
    /// Current write position (advances as data arrives).
    current: u64,
}

impl ReqwestFile {
    /// Range-download mode: download bytes `start..=end`.
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
        }
    }

    /// Streaming mode: unknown size, sequential append.
    pub fn new_streaming(
        save_file: Arc<Actor<FileSave>>,
        inner_status: Arc<DownloadInner>,
        client: reqwest::Client,
    ) -> Self {
        Self::new(save_file, inner_status, client, 0, 0)
    }

    // ── Range download ────────────────────────────────────────────────────────

    /// Download the assigned byte range, retrying up to 10 times with backoff.
    #[inline]
    pub async fn run(mut self) -> Result<()> {
        let mut attempt: u32 = 0;
        while !self.inner_status.is_finish() && self.current <= self.end {
            if !self.inner_status.is_start.load(Ordering::Acquire) {
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            let req = self
                .client
                .get(self.inner_status.url.as_str())
                .header(
                    reqwest::header::RANGE,
                    format!("bytes={}-{}", self.current, self.end),
                )
                .send();

            match timeout(Duration::from_secs(15), req).await {
                Ok(Ok(response))
                    if response.status() == StatusCode::OK
                        || response.status() == StatusCode::PARTIAL_CONTENT =>
                {
                    log::trace!(
                        "range {}-{} started (attempt {})",
                        self.current,
                        self.end,
                        attempt
                    );
                    if self.read_stream_inner(response, false).await? {
                        return Ok(()); // block complete
                    }
                    // Stream interrupted — retry with backoff.
                    attempt += 1;
                    sleep(retry_delay(attempt)).await;
                }
                Ok(Ok(response)) => {
                    let status = response.status();
                    attempt += 1;
                    if attempt >= 10 {
                        return Err(DownloadError::HttpStatusError(status.to_string()));
                    }
                    log::error!("range download status:{} retry:{}", status, attempt);
                    sleep(retry_delay(attempt)).await;
                }
                Ok(Err(err)) => {
                    attempt += 1;
                    if attempt >= 10 {
                        return Err(DownloadError::ReqwestError { source: err });
                    }
                    log::error!("range download error:{} retry:{}", err, attempt);
                    sleep(retry_delay(attempt)).await;
                }
                Err(_) => {
                    attempt += 1;
                    log::warn!(
                        "range download timeout (attempt {}), url:{}",
                        attempt,
                        self.inner_status.url
                    );
                    sleep(retry_delay(attempt)).await;
                }
            }
        }
        Ok(())
    }

    /// Run using an already-open response for the first attempt, then fall back to `run`.
    #[inline]
    pub async fn run_once(mut self, response: Response) -> Result<()> {
        if self.read_stream_inner(response, false).await? {
            Ok(())
        } else {
            self.run().await
        }
    }

    // ── Streaming download ────────────────────────────────────────────────────

    /// Streaming download for unknown file sizes — sequential append, no range headers.
    #[inline]
    pub async fn run_streaming(mut self, response: Response) -> Result<()> {
        if self.read_stream_inner(response, true).await? {
            return Ok(());
        }
        // Stream interrupted — resume with range if possible.
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            sleep(retry_delay(attempt)).await;

            let req = self
                .client
                .get(self.inner_status.url.as_str())
                .header(reqwest::header::RANGE, format!("bytes={}-", self.current))
                .send();

            match timeout(Duration::from_secs(30), req).await {
                Ok(Ok(response)) => {
                    let status = response.status();
                    if status == StatusCode::OK || status == StatusCode::PARTIAL_CONTENT {
                        if self.read_stream_inner(response, true).await? {
                            return Ok(());
                        }
                    } else if status == StatusCode::RANGE_NOT_SATISFIABLE {
                        // Server doesn't support resume; keep what we have.
                        log::warn!(
                            "server rejected range resume; keeping {} bytes",
                            self.current
                        );
                        return Ok(());
                    } else if attempt >= 10 {
                        return Err(DownloadError::HttpStatusError(status.to_string()));
                    } else {
                        log::error!("streaming resume status:{} retry:{}", status, attempt);
                    }
                }
                Ok(Err(err)) => {
                    if attempt >= 10 {
                        return Err(DownloadError::ReqwestError { source: err });
                    }
                    log::error!("streaming resume error:{} retry:{}", err, attempt);
                }
                Err(_) => {
                    log::warn!(
                        "streaming resume timeout (attempt {}), url:{}",
                        attempt,
                        self.inner_status.url
                    );
                    if attempt >= 10 {
                        return Ok(()); // give up but don't hard-fail
                    }
                }
            }
        }
    }

    // ── Internal stream reader ────────────────────────────────────────────────

    /// Read a response body chunk-by-chunk.
    ///
    /// - `append = false`: write at `self.current` offset (range mode).
    /// - `append = true`:  append sequentially (streaming mode).
    ///
    /// Returns `true` when the server closes the stream cleanly (block complete).
    /// Returns `false` on timeout, network error, or suspend signal.
    async fn read_stream_inner(&mut self, response: Response, append: bool) -> Result<bool> {
        let mut stream = response.bytes_stream();
        let completed = loop {
            match timeout(Duration::from_secs(10), stream.next()).await {
                Ok(Some(Ok(buf))) => {
                    let len = buf.len() as u64;
                    if append {
                        self.save_file.write_all(&buf).await?;
                    } else {
                        self.save_file
                            .write_all_by_offset(&buf, self.current)
                            .await?;
                    }
                    self.current += len;
                    self.inner_status.add_down_size(len);
                    if !self.inner_status.is_start.load(Ordering::Acquire) {
                        log::debug!("download suspended at offset {}", self.current);
                        break false;
                    }
                }
                Ok(Some(Err(err))) => {
                    log::error!("chunk error url:{} err:{}", self.inner_status.url, err);
                    break false;
                }
                Ok(None) => {
                    log::trace!(
                        "stream closed url:{} offset:{}-{}",
                        self.inner_status.url,
                        self.start,
                        self.current
                    );
                    break true;
                }
                Err(_) => {
                    log::warn!(
                        "chunk read timeout url:{} offset:{}",
                        self.inner_status.url,
                        self.current
                    );
                    break false;
                }
            }
        };
        Ok(completed)
    }
}
