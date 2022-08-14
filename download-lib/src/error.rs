use thiserror::Error;

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
}

pub type Result<T> = std::result::Result<T, DownloadError>;
