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



#[derive(Default)]
pub struct DownloadItem{
    down_core: OnceCell<DownloadFile>,
    error: OnceCell<DownloadError>,
}

/// Download handler context
pub struct DownloadHandler {
    _runtime: Runtime,
    items:slab::Slab<Arc<DownloadItem>>
}



#[no_mangle]
pub extern "C" fn durl_create(thread_count:u32) -> *mut DownloadHandler {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(thread_count as usize)
        .enable_all()
        .build()
        .expect("tokio runtime fail");

    Box::into_raw(Box::new(DownloadHandler {
        _runtime: runtime,
        items: Default::default()
    }))
}

/// # Safety
/// free DownloadHandler
#[no_mangle]
pub unsafe extern "C" fn durl_release(handler: *mut DownloadHandler) {
    let handler = Box::from_raw(handler);
    drop(handler)
}

/// clean key money
#[no_mangle]
pub extern "C" fn durl_clean(handler: &mut DownloadHandler,key:u64){
    handler.items.remove(key as usize);
}


/// # Safety
/// start now download url file to path,task is concurrent quantity
/// if return nullptr use get_logs look log content analysis quest.
/// url and path is cstr end is '\0',otherwise it will Undefined behavior
#[no_mangle]
pub unsafe extern "C" fn durl_start(
    handler: &mut DownloadHandler,
    url: *const c_char,
    path: *const c_char,
    task: u64,
    block: u64,
)->u64 {
    let url = CStr::from_ptr(url).to_str().unwrap().to_string();
    let path = CStr::from_ptr(path).to_str().unwrap().to_string();
    let save_path = PathBuf::from(path);


    let item=Arc::new(DownloadItem::default());
    let item_ptr=item.clone();
    let key= handler.items.insert(item);
    handler._runtime.spawn(async move {
        match DownloadFile::start_download(url, save_path, task, block).await {
            Ok(download) => {
                let _ = item_ptr.down_core.set(download);
            }
            Err(err) => {
                let _ = item_ptr.error.set(err);
            }
        }
    });

    key as u64
}

/// get download is start
#[no_mangle]
pub extern "C" fn durl_is_downloading( handler: &mut DownloadHandler,key:u64) -> bool {
    if let Some(item)=handler.items.get(key as usize){
        if let Some(download) = item.down_core.get() {
            if download.is_error() || item.error.initialized() {
                true
            } else {
                download.is_start()
            }
        } else {
            item.error.initialized()
        }
    }else{
        false
    }
}

#[no_mangle]
pub extern "C" fn durl_is_downloading_finish(handler: &DownloadHandler,key:u64) -> bool {
    if let Some(item)=handler.items.get(key as usize) {
        if let Some(download) = item.down_core.get() {
            download.is_finish()
        } else {
            false
        }
    }else{
        false
    }
}

/// get state
/// if error return error msg len
#[no_mangle]
pub extern "C" fn durl_get_state(
    handler: &DownloadHandler,
    key:u64,
    size: &mut u64,
    down_size: &mut u64,
    err_code: &mut i32,
) -> u32 {
    if let Some(item)=handler.items.get(key as usize) {
        if let Some(err) = item.error.get() {
            let len = cstr!(err).len();
            *err_code = err.into();
            len as u32
        } else if let Some(download) = item.down_core.get() {
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
    }else{
        *size = 0;
        *down_size = 0;
        *err_code = 0;
        0
    }
}

/// # Safety
/// get error msg string
#[no_mangle]
pub unsafe extern "C" fn durl_get_error_str(handler: &DownloadHandler,key:u64, msg: *mut c_char) {
    if let Some(item)=handler.items.get(key as usize) {
        if let Some(err) = item.error.get() {
            let err_msg = cstr!(err);
            let len = err_msg.len();
            msg.copy_from(err_msg.as_ptr().cast(), len as usize);
        } else if let Some(download) = item.down_core.get() {
            if let Some(err) = download.get_error() {
                let err_msg = cstr!(err);
                let len = err_msg.len();
                msg.copy_from(err_msg.as_ptr().cast(), len as usize);
            }
        }
    }
}
