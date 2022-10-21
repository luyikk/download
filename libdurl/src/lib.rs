extern crate alloc;
extern crate core;

use download_lib::DownloadFile;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr::null_mut;
use tokio::runtime::Runtime;


#[macro_export]
macro_rules! cstr {
    ($str:expr) => {
        format!("{}\0", $str)
    };
}

/// Download handler context
#[repr(C)]
pub struct DownloadHandler {
    _runtime: Runtime,
    down_core: DownloadFile,
}

/// start now download url file to path,task is concurrent quantity
/// if return nullptr use get_logs look log content analysis quest.
/// # Safety
/// url and path is cstr end is '\0',otherwise it will Undefined behavior
#[no_mangle]
pub unsafe extern "C" fn start_now(
    url: *const c_char,
    path: *const c_char,
    task: u64,
) -> *mut DownloadHandler {
    let url = CStr::from_ptr(url).to_str().unwrap().to_string();
    let path = CStr::from_ptr(path).to_str().unwrap().to_string();
    match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(runtime) => {
            let save_path = PathBuf::from(path);
            match runtime
                .block_on(async move { DownloadFile::start_download(url, save_path, task).await })
            {
                Ok(down_core) => Box::into_raw(Box::new(DownloadHandler {
                    _runtime: runtime,
                    down_core,
                })),
                Err(err) => {
                    println!("error:{:?}", err);
                    null_mut()
                }
            }
        }
        Err(err) => {
            println!("error:{}", err);
            null_mut()
        }
    }
}


/// get url str, return copy c_char len
/// # Safety
/// if args url ptr real len !=ptr_len will Undefined behavior
#[no_mangle]
pub unsafe extern "C" fn get_url(
    handler: &DownloadHandler,
    url_ptr: *mut c_char,
    url_len: u64,
) -> u64 {
    let url = cstr!(handler.down_core.url());
    let len = url.len();
    if url_len >= url.len() as u64 {
        url_ptr.copy_from(url.as_ptr().cast(), len);
    }
    len as u64
}

/// get url str, return copy c_char len
/// # Safety
/// if args url ptr real len !=ptr_len will Undefined behavior
#[no_mangle]
pub unsafe extern "C" fn get_save_path(
    handler: &DownloadHandler,
    path_ptr: *mut c_char,
    path_len: u64,
) -> u64 {
    let url = cstr!(handler.down_core.get_real_file_path());
    let len = (url.len() as u64).min(path_len);
    path_ptr.copy_from(url.as_ptr().cast(), len as usize);
    len
}



/// get download is start
#[no_mangle]
pub extern "C" fn is_start(handler: &DownloadHandler) -> bool {
    handler.down_core.is_start()
}

/// get download is finish
#[no_mangle]
pub extern "C" fn is_finish(handler: &DownloadHandler) -> bool {
    handler.down_core.is_finish()
}


/// get download file size
#[no_mangle]
pub extern "C" fn get_size(handler: &DownloadHandler)->u64{
    handler.down_core.size()
}

/// get download is error
/// if true use get_logs look log content analysis quest.
#[no_mangle]
pub extern "C" fn is_error(handler: &DownloadHandler) -> bool {
    handler.down_core.is_error()
}

/// get complete percent 0.00%-100.00%
#[no_mangle]
pub extern "C" fn get_percent_complete(handler: &DownloadHandler) -> f64 {
    handler.down_core.get_status().get_percent_complete()
}

/// get current download byte by sec
#[no_mangle]
pub extern "C" fn get_byte_sec(handler: &DownloadHandler) -> u64 {
    handler.down_core.get_status().get_byte_sec()
}

/// suspend download
#[no_mangle]
pub extern "C" fn suspend(handler: &DownloadHandler) {
    handler.down_core.suspend();
}

/// restart download
#[no_mangle]
pub extern "C" fn restart(handler: &DownloadHandler) {
    handler.down_core.restart();
}

#[no_mangle]
pub extern "C" fn get_state(handler: &DownloadHandler, size: &mut u64, down_size:&mut u64, err_code:&mut i32) ->u32{
    *size=handler.down_core.size();
    *down_size=handler.down_core.get_down_size();

    0
}
