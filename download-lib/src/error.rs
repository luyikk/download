use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("reqwest error->{source:?}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("io error->{source:?}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("not get file size ->{0:?}")]
    NotGetFileSize(reqwest::Url),
    #[error("save file is finish->{0:?}")]
    SaveFileFinish(String),
    #[error("not get file name ->{0:?}")]
    NotFileName(reqwest::Url),
    #[error("http error:{0}")]
    HttpStatusError(String),
    #[error("async join error:{0}")]
    JoinInError(JoinError),
}

impl From<&DownloadError> for i32 {
    fn from(v: &DownloadError) -> Self {
        match v {
            DownloadError::ReqwestError { .. } => 1,
            DownloadError::IoError { .. } => 2,
            DownloadError::NotGetFileSize { .. } => 3,
            DownloadError::SaveFileFinish { .. } => 4,
            DownloadError::NotFileName { .. } => 5,
            DownloadError::HttpStatusError { .. } => 6,
            DownloadError::JoinInError { .. } => 7,
        }
    }
}

pub type Result<T> = std::result::Result<T, DownloadError>;
