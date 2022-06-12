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
    #[error("not get file name ->{0:?}")]
    NotFileName(reqwest::Url),
}

pub type Result<T> = std::result::Result<T, DownloadError>;
