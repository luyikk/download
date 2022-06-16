use super::error::{DownloadError, Result};
use super::file_save::{FileSave, IFileSave};
use super::DownloadInner;
use crate::StatusCode;
use aqueue::Actor;
use futures_util::StreamExt;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{timeout,sleep};


/// http download file
pub(crate) struct ReqwestFile {
    save_file: Arc<Actor<FileSave>>,
    inner_status: Arc<DownloadInner>,
    end: u64,
    current: u64,
}

impl ReqwestFile {
    pub fn new(
        save_file: Arc<Actor<FileSave>>,
        inner_status: Arc<DownloadInner>,
        start: u64,
        end: u64,
    ) -> Self {
        Self {
            save_file,
            inner_status,
            end,
            current: start,
        }
    }

    #[inline]
    pub async fn run(&mut self) -> Result<()> {
        while !self.inner_status.is_finish() && self.current < self.end {
            if !self.inner_status.is_start.load(Ordering::Acquire) {
                sleep(Duration::from_secs(1)).await
            } else {
                're: for i in (0..10).rev() {
                    match timeout(Duration::from_secs(15), reqwest::Client::new()
                        .get(self.inner_status.url.as_str())
                        .header(
                            reqwest::header::RANGE,
                            format!("bytes={}-{}", self.current, self.end),
                        )
                        .send()).await
                    {
                        Ok(Ok(response)) => {
                            if response.status() == StatusCode::OK || response.status() ==  StatusCode::PARTIAL_CONTENT {
                                log::trace!(
                                    "start download url block:{} start:{} end:{}",
                                    self.inner_status.url,
                                    self.current,
                                    self.end
                                );
                                let mut stream = response.bytes_stream();
                                loop {
                                    match timeout(Duration::from_secs(10), stream.next()).await{
                                        Ok(Some(Ok(buf)))=> {
                                            self.save_file
                                                .write_all_by_offset(&buf, self.current)
                                                .await?;
                                            let len = buf.len() as u64;
                                            self.current += len;
                                            self.inner_status.add_down_size(len);
                                            if !self.inner_status.is_start() {
                                                log::debug!("is suspend");
                                                break;
                                            }
                                        },
                                        Ok(Some(Err(err)))=>{
                                            log::error!("download url:{} buff is error:{}",self.inner_status.url,err);
                                            break;
                                        }
                                        Ok(None)=>break,
                                        Err(_)=>{
                                            log::warn!("download url:{} time out", self.inner_status.url);
                                            break;
                                        }
                                    }
                                }
                                break 're;
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
                                return Err(DownloadError::ReqwestError {
                                    source: err,
                                    backtrace: std::backtrace::Backtrace::capture(),
                                });
                            }
                        },
                        Err(_)=>{
                            log::warn!("get url:{} response time out", self.inner_status.url);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
