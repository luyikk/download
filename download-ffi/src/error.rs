use download_lib::DownloadError;
use interoptopus::ffi_type;
use interoptopus::patterns::result::FFIError;

#[ffi_type(patterns(ffi_error))]
#[repr(C)]
#[derive(Debug)]
pub enum DownLoadFFIError {
    Ok = 0,
    NullPassed = 1,
    Panic = 2,
    DownError = 3,
}

// Gives special meaning to some of your error variants.
impl FFIError for DownLoadFFIError {
    const SUCCESS: Self = Self::Ok;
    const NULL: Self = Self::NullPassed;
    const PANIC: Self = Self::Panic;
}

impl From<DownloadError> for DownLoadFFIError {
    fn from(err: DownloadError) -> Self {
        log::error!("error:{:?}", err);
        DownLoadFFIError::DownError
    }
}

impl From<anyhow::Error> for DownLoadFFIError {
    fn from(err: anyhow::Error) -> Self {
        log::error!("error:{:?}", err);
        DownLoadFFIError::DownError
    }
}
