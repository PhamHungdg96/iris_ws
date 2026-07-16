//! IRIS Bridge - flutter_rust_bridge API layer.
//!
//! This crate wraps iris_core functionality and exposes it
//! via C-compatible FFI wire functions for Dart.

mod api;

use api::iris_api::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── Helper functions ──────────────────────────────────────────────

fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

unsafe fn from_c_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

// ─── Wire Functions (C FFI for flutter_rust_bridge) ────────────────

#[no_mangle]
pub extern "C" fn wire_iris_init(
    device_name: *const c_char,
    platform: *const c_char,
    download_dir: *const c_char,
) -> *mut c_char {
    let name = unsafe { from_c_str(device_name) };
    let plat = unsafe { from_c_str(platform) };
    let dir = unsafe { from_c_str(download_dir) };
    to_c_string(&iris_init(name.to_string(), plat.to_string(), dir.to_string()))
}

#[no_mangle]
pub extern "C" fn wire_iris_start() -> *mut c_char {
    to_c_string(&iris_start())
}

#[no_mangle]
pub extern "C" fn wire_iris_restart_with_port(tcp_port: u16) -> *mut c_char {
    to_c_string(&iris_restart_with_port(tcp_port))
}

#[no_mangle]
pub extern "C" fn wire_iris_get_devices() -> *mut c_char {
    to_c_string(&iris_get_devices())
}

#[no_mangle]
pub extern "C" fn wire_iris_connect(address: *const c_char) -> *mut c_char {
    let addr = unsafe { from_c_str(address) };
    to_c_string(&iris_connect(addr.to_string()))
}

#[no_mangle]
pub extern "C" fn wire_iris_capture_frame() -> *mut c_char {
    to_c_string(&iris_capture_frame())
}

#[no_mangle]
pub extern "C" fn wire_iris_encode_frame(
    rgba_data: *const u8,
    data_len: u32,
    width: u32,
    height: u32,
    encoding_type: u8,
) -> *mut c_char {
    let data = unsafe { std::slice::from_raw_parts(rgba_data, data_len as usize) };
    to_c_string(&iris_encode_frame(data.to_vec(), width, height, encoding_type))
}

#[no_mangle]
pub extern "C" fn wire_iris_decode_frame(
    base64_data: *const c_char,
    width: u32,
    height: u32,
    encoding_type: u8,
) -> *mut c_char {
    let b64 = unsafe { from_c_str(base64_data) };
    to_c_string(&iris_decode_frame(b64.to_string(), width, height, encoding_type))
}

#[no_mangle]
pub extern "C" fn wire_iris_get_version() -> *mut c_char {
    to_c_string(&iris_get_version())
}

#[no_mangle]
pub extern "C" fn wire_iris_list_files(dir_path: *const c_char) -> *mut c_char {
    let path = unsafe { from_c_str(dir_path) };
    to_c_string(&iris_list_files(path.to_string()))
}

#[no_mangle]
pub extern "C" fn wire_iris_display_dimensions() -> *mut c_char {
    to_c_string(&iris_display_dimensions())
}

#[no_mangle]
pub extern "C" fn wire_iris_udp_send(
    target: *const c_char,
    encoded_b64: *const c_char,
) -> *mut c_char {
    let t = unsafe { from_c_str(target) };
    let b = unsafe { from_c_str(encoded_b64) };
    to_c_string(&iris_udp_send(t.to_string(), b.to_string()))
}

#[no_mangle]
pub extern "C" fn wire_iris_udp_receive_latest() -> *mut c_char {
    to_c_string(&iris_udp_receive_latest())
}

/// Free a string returned by any wire_* function.
#[no_mangle]
pub extern "C" fn iris_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
