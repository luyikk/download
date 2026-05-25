extern crate alloc;
extern crate core;

use download_lib::{DownloadError, DownloadFile};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;
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
    items: slab::Slab<Arc<DownloadItem>>,
}

fn copy_cstr(dst: *mut c_char, text: &str) -> u32 {
    if dst.is_null() {
        return 0;
    }
    let bytes = text.as_bytes();
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), dst, bytes.len());
        *dst.add(bytes.len()) = 0;
    }
    (bytes.len() + 1) as u32
}

unsafe fn from_cstr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string())
}

fn spawn_download(
    handler: &mut DownloadHandler,
    url: String,
    save_path: PathBuf,
    task: u64,
    block: u64,
    file_name: Option<String>,
) -> u64 {
    let item = Arc::new(DownloadItem::default());
    let item_ptr = item.clone();
    let key = handler.items.insert(item);
    handler._runtime.spawn(async move {
        match DownloadFile::start_download(url, save_path, task, block, file_name).await {
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



#[no_mangle]
pub extern "C" fn durl_create(thread_count:u32) -> *mut DownloadHandler {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(thread_count as usize)
        .enable_all()
        .build()
        .expect("tokio runtime fail");

    Box::into_raw(Box::new(DownloadHandler {
        _runtime: runtime,
        items: Default::default(),
    }))
}

/// # Safety
/// free DownloadHandler
#[no_mangle]
pub unsafe extern "C" fn durl_release(handler: *mut DownloadHandler) {
    if handler.is_null() {
        return;
    }
    let handler = Box::from_raw(handler);
    drop(handler)
}

/// clean key money
#[no_mangle]
pub extern "C" fn durl_clean(handler: &mut DownloadHandler,key:u64){
    if handler.items.contains(key as usize) {
        handler.items.remove(key as usize);
    }
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
    let url = match from_cstr(url) {
        Some(v) => v,
        None => return u64::MAX,
    };
    let path = match from_cstr(path) {
        Some(v) => v,
        None => return u64::MAX,
    };
    let save_path = PathBuf::from(path);

    spawn_download(handler, url, save_path, task, block, None)
}

/// # Safety
/// start now download url file to path,task is concurrent quantity
/// if return nullptr use get_logs look log content analysis quest.
/// url and path is cstr end is '\0',otherwise it will Undefined behavior
#[no_mangle]
pub unsafe extern "C" fn durl_start_file_name(
    handler: &mut DownloadHandler,
    url: *const c_char,
    path: *const c_char,
    file_name: *const c_char,
    task: u64,
    block: u64,
)->u64 {
    let url = match from_cstr(url) {
        Some(v) => v,
        None => return u64::MAX,
    };
    let path = match from_cstr(path) {
        Some(v) => v,
        None => return u64::MAX,
    };
    let file_name = match from_cstr(file_name) {
        Some(v) => v,
        None => return u64::MAX,
    };
    let save_path = PathBuf::from(path);

    spawn_download(handler, url, save_path, task, block, Some(file_name))
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

#[no_mangle]
pub extern "C" fn durl_suspend(handler: &DownloadHandler, key: u64) {
    if let Some(item) = handler.items.get(key as usize) {
        if let Some(download) = item.down_core.get() {
            download.suspend();
        }
    }
}

#[no_mangle]
pub extern "C" fn durl_restart(handler: &DownloadHandler, key: u64) {
    if let Some(item) = handler.items.get(key as usize) {
        if let Some(download) = item.down_core.get() {
            download.restart();
        }
    }
}

/// # Safety
/// get temp download save path (ends with .dd), returns copied c-string length
#[no_mangle]
pub unsafe extern "C" fn durl_get_save_file_path(handler: &DownloadHandler, key: u64, msg: *mut c_char) -> u32 {
    if let Some(item) = handler.items.get(key as usize) {
        if let Some(download) = item.down_core.get() {
            return copy_cstr(msg, &download.get_save_file_path());
        }
    }
    0
}

/// # Safety
/// get final file path, returns copied c-string length
#[no_mangle]
pub unsafe extern "C" fn durl_get_real_file_path(handler: &DownloadHandler, key: u64, msg: *mut c_char) -> u32 {
    if let Some(item) = handler.items.get(key as usize) {
        if let Some(download) = item.down_core.get() {
            return copy_cstr(msg, &download.get_real_file_path());
        }
    }
    0
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
            let _ = copy_cstr(msg, &err_msg);
        } else if let Some(download) = item.down_core.get() {
            if let Some(err) = download.get_error() {
                let err_msg = cstr!(err);
                let _ = copy_cstr(msg, &err_msg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn copy_cstr_writes_null_terminated_data() {
        let mut buf = vec![0_i8; 16];
        let len = copy_cstr(buf.as_mut_ptr(), "abc");
        assert_eq!(len, 4);
        assert_eq!(buf[0], b'a' as i8);
        assert_eq!(buf[1], b'b' as i8);
        assert_eq!(buf[2], b'c' as i8);
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn copy_cstr_null_ptr_returns_zero() {
        let len = copy_cstr(ptr::null_mut(), "abc");
        assert_eq!(len, 0);
    }

    #[test]
    fn clean_invalid_key_no_panic() {
        let handler = unsafe { &mut *durl_create(1) };
        durl_clean(handler, 999);
        unsafe { durl_release(handler) };
    }

    #[test]
    fn start_with_null_url_returns_sentinel() {
        let handler = unsafe { &mut *durl_create(1) };
        let path = CString::new("./").unwrap();
        let key = unsafe { durl_start(handler, ptr::null(), path.as_ptr(), 1, 1024) };
        assert_eq!(key, u64::MAX);
        unsafe { durl_release(handler) };
    }

    #[test]
    fn get_real_path_invalid_key_returns_zero() {
        let handler = unsafe { &mut *durl_create(1) };
        let mut buf = vec![0_i8; 260];
        let len = unsafe { durl_get_real_file_path(handler, 999, buf.as_mut_ptr()) };
        assert_eq!(len, 0);
        unsafe { durl_release(handler) };
    }
}
