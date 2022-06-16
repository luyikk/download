extern crate core;

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::{CStr, CString};
use std::ptr::null_mut;

#[test]
fn test_init() {
    assert!(libdurl::init());
    unsafe {
        let url = CString::new("https://crates.io/assets/Cargo-Logo-Small.png").unwrap();
        let path = CString::new("./1").unwrap();
        libdurl::start_now(url.as_ptr(), path.as_ptr(), 10);
        let len = libdurl::get_logs(null_mut(), 0);
        //assert_eq!(34,len);

        let layout = Layout::from_size_align(len as usize, 8).unwrap();
        let mem = alloc(layout);
        assert_eq!(libdurl::get_logs(mem.cast(), len), len);
        let str = CStr::from_ptr(mem.cast()).to_str().unwrap();
        println!("{}", str);
        dealloc(mem, layout);
    }
}

#[test]
fn test_url() {
    //assert!(libdurl::init());
    unsafe {
        let url = CString::new("https://crates.io/assets/Cargo-Logo-Small.png").unwrap();
        let path = CString::new("./2").unwrap();
        let hd = libdurl::start_now(url.as_ptr(), path.as_ptr(), 10);
        let len = libdurl::get_url(&*hd, null_mut(), 0);
        //assert_eq!(34,len);

        let layout = Layout::from_size_align(len as usize, 8).unwrap();
        let mem = alloc(layout);
        assert_eq!(libdurl::get_url(&*hd, mem.cast(), len), len);
        let str = CStr::from_ptr(mem.cast()).to_str().unwrap();
        println!("{}", str);
        dealloc(mem, layout);
    }
}
