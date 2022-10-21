extern crate alloc;
extern crate core;

use download_lib::{DownloadError, DownloadFile};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

#[macro_export]
macro_rules! cstr {
    ($str:expr) => {
        format!("{}\0", $str)
    };
}

/// Download handler context

pub struct DownloadHandler {
    _runtime: Runtime,
    down_core: Arc<OnceCell<DownloadFile>>,
    error: Arc<OnceCell<DownloadError>>,
}

#[no_mangle]
pub extern "C" fn rd_create() -> *mut DownloadHandler {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime fail");

    Box::into_raw(Box::new(DownloadHandler {
        _runtime: runtime,
        down_core: Default::default(),
        error: Default::default(),
    }))
}

/// # Safety
/// free DownloadHandler
#[no_mangle]
pub unsafe extern "C" fn rd_release(handler: *mut DownloadHandler) {
    let handler = Box::from_raw(handler);
    drop(handler)
}

/// # Safety
/// start now download url file to path,task is concurrent quantity
/// if return nullptr use get_logs look log content analysis quest.
/// url and path is cstr end is '\0',otherwise it will Undefined behavior
#[no_mangle]
pub unsafe extern "C" fn rd_start(
    handler: &mut DownloadHandler,
    url: *const c_char,
    path: *const c_char,
    task: u64,
    block:u64
) {
    if !handler.down_core.initialized() && !handler.error.initialized() {
        let url = CStr::from_ptr(url).to_str().unwrap().to_string();
        let path = CStr::from_ptr(path).to_str().unwrap().to_string();
        let save_path = PathBuf::from(path);

        let down_core_ptr = handler.down_core.clone();
        let error_ptr = handler.error.clone();
        handler._runtime.spawn(async move {
            match DownloadFile::start_download(url, save_path, task,block).await {
                Ok(download) => {
                    let _ = down_core_ptr.set(download);
                }
                Err(err) => {
                    let _ = error_ptr.set(err);
                }
            }
        });
    }
}

/// get download is start
#[no_mangle]
pub extern "C" fn rd_is_downloading(handler: &DownloadHandler) -> bool {
    if let Some(download) = handler.down_core.get() {
        if download.is_error() || handler.error.initialized(){
            true
        }else{
            download.is_start()
        }
    } else {
        handler.error.initialized()
    }
}

/// get state
/// if error return error msg len
#[no_mangle]
pub extern "C" fn rd_get_state(
    handler: &DownloadHandler,
    size: &mut u64,
    down_size: &mut u64,
    err_code: &mut i32,
) -> u32 {
    if let Some(err) = handler.error.get() {
        let len = cstr!(err).len();
        *err_code = err.into();
        len as u32
    } else if let Some(download) = handler.down_core.get() {
        *size = download.size();
        *down_size = download.get_down_size();
        if let Some(err) = download.get_error() {
            let len = cstr!(err).len();
            *err_code = err.into();
            len as u32
        } else {
            *err_code = 0;
            0
        }
    } else {
        *size = 0;
        *down_size = 0;
        *err_code = 0;
        0
    }
}

/// # Safety
/// get error msg string
#[no_mangle]
pub unsafe extern "C" fn rd_get_error_str(handler: &DownloadHandler, msg: *mut c_char) {
    if let Some(err) = handler.error.get() {
        let err_msg = cstr!(err);
        let len = err_msg.len();
        msg.copy_from(err_msg.as_ptr().cast(), len as usize);
    } else if let Some(download) = handler.down_core.get() {
        if let Some(err) = download.get_error() {
            let err_msg = cstr!(err);
            let len = err_msg.len();
            msg.copy_from(err_msg.as_ptr().cast(), len as usize);
        }
    }
}
