//! C-compatible FFI interface for Flutter/Dart integration.
//!
//! All functions use `#[no_mangle]` and `extern "C"` for dart:ffi.
//! Complex data is passed via JSON strings through the FFI boundary
//! to keep the interface simple and maintainable.
//!
//! Memory management: caller owns all returned strings, must free via `iris_free_string`.

use crate::discovery::DiscoveryService;
use crate::protocol::{FrameEncoding, TcpMessage};
use crate::screen::encoder;
use crate::transport::tcp::TcpTransport;
use crate::transport::udp::UdpTransport;
use log::info;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use base64::Engine as _;

// ─── Global State ──────────────────────────────────────────────────

static ENGINE: Lazy<Mutex<Option<IrisEngine>>> = Lazy::new(|| Mutex::new(None));
static RT: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime")
});

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    RT.block_on(f)
}

struct IrisEngine {
    discovery: Option<DiscoveryService>,
    tcp: Option<TcpTransport>,
    udp: Option<UdpTransport>,
    device_name: String,
    device_id: String,
    platform: String,
}

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

// ─── FFI Exports ───────────────────────────────────────────────────

/// Initialize the IRIS engine. Must be called before any other function.
#[no_mangle]
pub extern "C" fn iris_init(
    device_name: *const c_char,
    platform: *const c_char,
    _download_dir: *const c_char,
) -> *mut c_char {
    let name = unsafe { from_c_str(device_name) };
    let plat = unsafe { from_c_str(platform) };

    let device_id = format!("iris-{}", uuid::Uuid::new_v4());

    let engine = IrisEngine {
        discovery: None,
        tcp: None,
        udp: None,
        device_name: name.to_string(),
        device_id: device_id.clone(),
        platform: plat.to_string(),
    };

    let mut guard = ENGINE.lock().unwrap();
    *guard = Some(engine);

    info!("IRIS engine initialized: {} on {}", name, plat);
    let result = serde_json::json!({
        "success": true,
        "device_id": device_id,
    });
    to_c_string(&result.to_string())
}

/// Start discovery + transport services.
#[no_mangle]
pub extern "C" fn iris_start() -> *mut c_char {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let mut guard = ENGINE.lock().unwrap();
        let engine = guard.as_mut().ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        // Start UDP transport
        let udp = block_on(UdpTransport::bind())?;
        engine.udp = Some(udp);

        // Start TCP transport
        let tcp = block_on(TcpTransport::bind())?;
        engine.tcp = Some(tcp);

        // Start discovery
        let discovery = DiscoveryService::new(&engine.device_name, &engine.platform)?;
        discovery.start(
            &engine.device_name,
            &engine.platform,
            crate::protocol::TCP_PORT,
            &engine.device_id,
        )?;
        engine.discovery = Some(discovery);

        Ok(serde_json::json!({"success": true}))
    })();

    match result {
        Ok(v) => to_c_string(&v.to_string()),
        Err(e) => to_c_string(&serde_json::json!({"success": false, "error": e.to_string()}).to_string()),
    }
}

/// Get list of discovered devices as JSON array.
#[no_mangle]
pub extern "C" fn iris_get_devices() -> *mut c_char {
    let guard = ENGINE.lock().unwrap();
    let devices = guard
        .as_ref()
        .and_then(|e| e.discovery.as_ref())
        .map(|d| d.get_devices())
        .unwrap_or_default();

    let json = serde_json::to_string(&devices).unwrap_or_else(|_| "[]".to_string());
    to_c_string(&json)
}

/// Connect to a remote device via TCP.
#[no_mangle]
pub extern "C" fn iris_connect(address: *const c_char) -> *mut c_char {
    let addr_str = unsafe { from_c_str(address) };

    let result = (|| -> anyhow::Result<serde_json::Value> {
        let addr: std::net::SocketAddr = addr_str.parse()?;
        let mut conn = block_on(TcpTransport::connect(addr))?;

        let guard = ENGINE.lock().unwrap();
        let engine = guard.as_ref().ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        let hello = TcpMessage::Hello(crate::protocol::HelloInfo {
            device_name: engine.device_name.clone(),
            platform: engine.platform.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: crate::protocol::Capabilities {
                screen_share: true,
                file_transfer: true,
                max_resolution: (1920, 1080),
            },
        });

        block_on(conn.send(&hello))?;

        Ok(serde_json::json!({"success": true}))
    })();

    match result {
        Ok(v) => to_c_string(&v.to_string()),
        Err(e) => to_c_string(&serde_json::json!({"success": false, "error": e.to_string()}).to_string()),
    }
}

/// Encode raw RGBA frame data for UDP transmission.
/// encoding: 0=Raw, 1=Jpeg, 2=Lz4, 3=H264
#[no_mangle]
pub extern "C" fn iris_encode_frame(
    rgba_data: *const u8,
    data_len: u32,
    width: u32,
    height: u32,
    encoding_type: u8,
) -> *mut c_char {
    let encoding = match encoding_type {
        0 => FrameEncoding::RawRgba,
        1 => FrameEncoding::Jpeg,
        2 => FrameEncoding::Lz4,
        3 => FrameEncoding::H264,
        _ => FrameEncoding::Jpeg,
    };

    let data = unsafe { std::slice::from_raw_parts(rgba_data, data_len as usize) };
    let frame = crate::screen::capture::CapturedFrame {
        width,
        height,
        data: data.to_vec(),
        timestamp_ms: 0,
    };

    match encoder::encode_frame(&frame, encoding) {
        Ok(encoded) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&encoded);
            let result = serde_json::json!({
                "success": true,
                "data": b64,
                "size": encoded.len(),
            });
            to_c_string(&result.to_string())
        }
        Err(e) => {
            to_c_string(&serde_json::json!({"success": false, "error": e.to_string()}).to_string())
        }
    }
}

/// Decode received frame data for display.
#[no_mangle]
pub extern "C" fn iris_decode_frame(
    base64_data: *const c_char,
    width: u32,
    height: u32,
    encoding_type: u8,
) -> *mut c_char {
    let b64_str = unsafe { from_c_str(base64_data) };

    let encoding = match encoding_type {
        0 => FrameEncoding::RawRgba,
        1 => FrameEncoding::Jpeg,
        2 => FrameEncoding::Lz4,
        3 => FrameEncoding::H264,
        _ => FrameEncoding::Jpeg,
    };

    match base64::engine::general_purpose::STANDARD.decode(b64_str) {
        Ok(data) => match encoder::decode_frame(&data, encoding, width, height) {
            Ok(rgba) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
                let result = serde_json::json!({
                    "success": true,
                    "data": b64,
                    "width": width,
                    "height": height,
                });
                to_c_string(&result.to_string())
            }
            Err(e) => to_c_string(
                &serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
            ),
        },
        Err(e) => {
            to_c_string(&serde_json::json!({"success": false, "error": e.to_string()}).to_string())
        }
    }
}

/// Free a string returned by any iris_* function.
#[no_mangle]
pub extern "C" fn iris_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Get the IRIS protocol version.
#[no_mangle]
pub extern "C" fn iris_get_version() -> *mut c_char {
    let version = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": crate::protocol::PROTOCOL_VERSION,
        "udp_port": crate::protocol::UDP_PORT,
        "tcp_port": crate::protocol::TCP_PORT,
    });
    to_c_string(&version.to_string())
}
