extern crate core;

mod error;

use anyhow::Result;
use download_lib::DownloadFile;
use interoptopus::patterns::api_guard::APIVersion;
use interoptopus::patterns::string::AsciiPointer;
use interoptopus::{ffi_function, ffi_service, ffi_service_ctor, ffi_type, ffi_service_method, function, pattern, Inventory, InventoryBuilder, callback};
use log::LevelFilter;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::runtime::Runtime;

use crate::error::DownLoadFFIError;

#[macro_export]
macro_rules! cstr {
    ($str:expr) => {
        format!("{}\0", $str)
    };
}

#[ffi_function]
#[no_mangle]
pub extern "C" fn my_api_guard() -> APIVersion {
    inventory().into()
}

/// init log out console
#[ffi_function]
#[no_mangle]
pub extern "C" fn init_logger(){
    env_logger::builder()
        .filter_module("want", LevelFilter::Error)
        .filter_module("mio", LevelFilter::Error)
        .filter_level(LevelFilter::Trace)
        .init();
}

pub fn inventory() -> Inventory {
    InventoryBuilder::new()
        .register(function!(my_api_guard))
        .register(function!(init_logger))
        .register(pattern!(DUrl))
        .inventory()
}

callback!(GetUrlCallBack(url: AsciiPointer));
callback!(GetRealSavePathCallBack(path: AsciiPointer));

#[ffi_type(opaque)]
pub struct DUrl {
    _runtime: Runtime,
    down_core: DownloadFile,
}

#[ffi_service(error = "DownLoadFFIError")]
impl DUrl {

    #[ffi_service_ctor]
    pub fn start_now(url: AsciiPointer, save_path: AsciiPointer, task: u64) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let url = url.as_str()?;
        let save_path = PathBuf::from_str(save_path.as_str()?)?;
        let down_core = runtime
            .block_on(async move { DownloadFile::start_download(url, save_path, task).await })?;

        Ok(Self{
            _runtime:runtime,
            down_core
        })
    }

    /// get down url
    pub fn get_url(&self,url_callback:GetUrlCallBack)->Result<()>{
        let url=cstr!(self.down_core.url());
        url_callback.call(AsciiPointer::from_slice_with_nul(url.as_bytes())?);
        Ok(())
    }

    /// start down is 1
    /// not start down is 0
    #[ffi_service_method(on_panic = "undefined_behavior")]
    pub fn is_start(&self) -> u8{
        if self.down_core.is_start(){
            1
        }else{
            0
        }
    }

    /// download is finish is 1
    /// not finish is 0
    #[ffi_service_method(on_panic = "undefined_behavior")]
    pub fn is_finish(&self) -> u8 {
        if self.down_core.is_finish(){
            1
        }else{
            0
        }
    }

    /// get complete percent
    #[ffi_service_method(on_panic = "undefined_behavior")]
    pub fn get_percent_complete(&self) -> f64 {
        self.down_core.get_status().get_percent_complete()
    }

    /// computer bs
    #[ffi_service_method(on_panic = "undefined_behavior")]
    pub fn get_byte_sec(&self) -> u64{
        self.down_core.get_status().get_byte_sec()
    }

    /// get save file real path
    pub fn get_real_file_path(&self,get_callback:GetRealSavePathCallBack) ->Result<()>{
        let path=cstr!(self.down_core.get_real_file_path());
        get_callback.call(AsciiPointer::from_slice_with_nul(path.as_bytes())?);
        Ok(())
    }
    /// suspend download
    pub fn suspend(&self)->Result<()> {
        self.down_core.suspend();
        Ok(())
    }

    /// restart download
    pub fn restart(&self)->Result<()> {
        self.down_core.restart();
        Ok(())
    }
}
