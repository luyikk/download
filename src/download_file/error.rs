use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("reqwest error->{source:?}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
        backtrace: Backtrace,
    },
    #[error("io error->{source:?}")]
    IoError {
        #[from]
        source: std::io::Error,
        backtrace: Backtrace,
    },
    #[error("not get file size ->{0:?}")]
    NotGetFileSize(reqwest::Url),
    #[error("save file is finish->{0:?}")]
    SaveFileFinish(String),
}

pub type Result<T> = std::result::Result<T, DownloadError>;
