use super::file_save::{FileSave, IFileSave};
use super::DownloadInner;
use super::error::{Result,DownloadError};
use aqueue::Actor;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use futures_util::StreamExt;
use tokio::time::sleep;


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
    pub async fn run(&mut self)->Result<()>{
        while!self.inner_status.is_finish() && self.current<self.end {
            if !self.inner_status.is_start.load(Ordering::Acquire) {
                sleep(Duration::from_secs(1)).await
            }else {
               're: for i in (0..10).rev() {
                    match reqwest::Client::new().get(self.inner_status.url.as_str())
                        .header(reqwest::header::RANGE, format!("bytes={}-{}", self.current, self.end))
                        .send()
                        .await {
                        Ok(response) => {
                            log::trace!("start download url block:{} start:{} end:{}",self.inner_status.url,self.current,self.end);
                            let mut stream=response.bytes_stream();
                            while let Some(buff) = stream.next().await {
                                let buff = buff?;
                                self.save_file.write_all_by_offset(&buff, self.current).await?;
                                let len=buff.len() as u64;
                                self.current += len;
                                self.inner_status.add_down_size(len);
                                if !self.inner_status.is_start() {
                                    log::debug!("is suspend");
                                    break 're;
                                }
                            }
                            break 're;
                        },
                        Err(err) => {
                            if i>0 {
                                log::error!("download url:{} error:{err} retry:{i}",self.inner_status.url);
                            }else{
                                return Err(DownloadError::ReqwestError {source:err, backtrace: std::backtrace::Backtrace::capture() })
                            }
                        }
                    }
                }

            }
        }
        Ok(())
    }
}
