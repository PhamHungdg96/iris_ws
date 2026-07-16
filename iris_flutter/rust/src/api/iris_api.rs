//! IRIS API - Public functions exposed to Flutter via flutter_rust_bridge.
//!
//! All public functions in this module become Dart methods.
//! flutter_rust_bridge handles serialization automatically.

use iris_core::discovery::DiscoveryService;
use iris_core::protocol::{Capabilities, FrameEncoding, HelloInfo, TcpMessage};
use iris_core::screen::encoder;
use iris_core::screen::capture::{CapturedFrame, create_capture};
use iris_core::transport::tcp::TcpTransport;
use iris_core::transport::udp::UdpTransport;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use base64::Engine as _;

// ─── Global State ──────────────────────────────────────────────────

static ENGINE: Lazy<Mutex<Option<IrisEngine>>> = Lazy::new(|| Mutex::new(None));
/// Buffer for the latest received UDP frame (base64 RGBA + dimensions)
static LAST_UDP_FRAME: Lazy<Mutex<Option<(Vec<u8>, u32, u32)>>> = Lazy::new(|| Mutex::new(None));
fn block_on<F: std::future::Future + Send + 'static>(f: F) -> F::Output
where
    F::Output: Send,
{
    // Spawn on a dedicated thread with its own runtime
    // to avoid conflicts with Flutter's internal runtime.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create IRIS runtime");
        let result = rt.block_on(f);
        let _ = tx.send(result);
    });
    rx.recv().expect("IRIS runtime thread panicked")
}

struct IrisEngine {
    discovery: Option<DiscoveryService>,
    tcp: Option<TcpTransport>,
    udp: Option<Arc<UdpTransport>>,
    device_name: String,
    device_id: String,
    platform: String,
    #[allow(dead_code)]
    download_dir: String,
}

// ─── Init & Lifecycle ──────────────────────────────────────────────

/// Initialize the IRIS engine. Must be called first.
/// Returns the generated device ID.
pub fn iris_init(device_name: String, platform: String, download_dir: String) -> String {
    let device_id = format!("iris-{}", uuid::Uuid::new_v4());

    let engine = IrisEngine {
        discovery: None,
        tcp: None,
        udp: None,
        device_name,
        device_id: device_id.clone(),
        platform,
        download_dir,
    };

    let mut guard = ENGINE.lock().unwrap();
    *guard = Some(engine);

    let result = serde_json::json!({
        "success": true,
        "device_id": device_id,
    });
    result.to_string()
}

/// Start discovery + transport services.
pub fn iris_start() -> String {
    start_internal(None)
}

/// Restart transports on a specific TCP port.
/// UDP will be TCP port + 1 (or next available).
pub fn iris_restart_with_port(tcp_port: u16) -> String {
    start_internal(Some(tcp_port))
}

fn start_internal(tcp_port_hint: Option<u16>) -> String {
    let device_name;
    let platform;
    let device_id;

    {
        let mut guard = ENGINE.lock().unwrap();
        let engine = match guard.as_mut() {
            Some(e) => e,
            None => return serde_json::json!({"success": false, "error": "Not initialized"}).to_string(),
        };
        engine.udp = None;
        engine.tcp = None;
        engine.discovery = None;
        device_name = engine.device_name.clone();
        platform = engine.platform.clone();
        device_id = engine.device_id.clone();
    }

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("IRIS runtime");

        // Bind transports
        let (udp_result, tcp_result) = rt.block_on(async {
            let udp = if let Some(tcp_port) = tcp_port_hint {
                UdpTransport::bind_from(tcp_port + 1).await
            } else {
                UdpTransport::bind().await
            };
            let tcp = if let Some(tcp_port) = tcp_port_hint {
                TcpTransport::bind_from(tcp_port).await
            } else {
                TcpTransport::bind().await
            };
            (udp, tcp)
        });

        let udp = match udp_result {
            Ok(u) => u,
            Err(e) => { log::error!("UDP bind failed: {}", e); return; }
        };
        let tcp = match tcp_result {
            Ok(t) => t,
            Err(e) => { log::error!("TCP bind failed: {}", e); return; }
        };

        let udp_port = udp.local_addr().port();
        let tcp_port = tcp.local_addr().port();
        let udp_arc = Arc::new(udp);

        // ── TCP accept loop ──
        let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel();
        rt.spawn(async move {
            if let Err(e) = tcp.accept_loop(msg_tx).await {
                log::error!("TCP accept loop: {}", e);
            }
        });
        rt.spawn(async move {
            while let Some((msg, addr)) = msg_rx.recv().await {
                match msg {
                    TcpMessage::Hello(info) => log::info!("Hello from {} ({})", info.device_name, addr),
                    TcpMessage::Ping => log::debug!("Ping from {}", addr),
                    _ => log::debug!("TCP msg from {}: {:?}", addr, msg),
                }
            }
        });

        // ── UDP receive loop ──
        let udp_recv = udp_arc.clone();
        rt.spawn(async move {
            loop {
                match udp_recv.recv().await {
                    Ok((msg, _addr)) => {
                        if let iris_core::protocol::UdpMessage::FrameChunk(chunk) = msg {
                            if let Ok(mut guard) = LAST_UDP_FRAME.lock() {
                                *guard = Some((chunk.data, chunk.width, chunk.height));
                            }
                        }
                    }
                    Err(e) => log::error!("UDP recv error: {}", e),
                }
            }
        });

        // ── Store in engine ──
        if let Ok(mut guard) = ENGINE.lock() {
            if let Some(ref mut engine) = *guard {
                engine.udp = Some(udp_arc.clone());
            }
        }

        // ── Discovery ──
        match DiscoveryService::new(&device_name, &platform, &device_id) {
            Ok(discovery) => {
                if let Err(e) = discovery.start(&device_name, &platform, tcp_port, udp_port, &device_id) {
                    log::error!("Discovery start failed: {}", e);
                } else if let Ok(mut guard) = ENGINE.lock() {
                    if let Some(ref mut engine) = *guard {
                        engine.discovery = Some(discovery);
                    }
                }
            }
            Err(e) => log::error!("DiscoveryService::new: {}", e),
        }

        log::info!("IRIS listening TCP:{} UDP:{}", tcp_port, udp_port);
    });

    serde_json::json!({"success": true, "status": "starting"}).to_string()
}

// ─── Discovery ─────────────────────────────────────────────────────

/// Get list of discovered devices as JSON array.
pub fn iris_get_devices() -> String {
    let guard = ENGINE.lock().unwrap();
    let devices = guard
        .as_ref()
        .and_then(|e| e.discovery.as_ref())
        .map(|d| d.get_devices())
        .unwrap_or_default();

    serde_json::to_string(&devices).unwrap_or_else(|_| "[]".to_string())
}

// ─── Connection ────────────────────────────────────────────────────

/// Connect to a remote device via TCP.
pub fn iris_connect(address: String) -> String {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let addr: std::net::SocketAddr = address.parse()?;
        let mut conn = block_on(TcpTransport::connect(addr))?;

        let guard = ENGINE.lock().unwrap();
        let engine = guard.as_ref().ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        let hello = TcpMessage::Hello(HelloInfo {
            device_name: engine.device_name.clone(),
            platform: engine.platform.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: Capabilities {
                screen_share: true,
                file_transfer: true,
                max_resolution: (1920, 1080),
            },
        });
        drop(guard);

        // Clone everything to satisfy 'static bound
        let hello_data = hello.to_bytes();
        block_on(async move {
            conn.send_raw(&hello_data).await
        })?;

        Ok(serde_json::json!({"success": true}))
    })();

    match result {
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
    }
}

// ─── Screen Capture ────────────────────────────────────────────────

/// Capture a single frame from the primary display.
/// Returns JSON with base64-encoded RGBA data.
pub fn iris_capture_frame() -> String {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let mut capturer = create_capture()?;
        let frame = capturer.capture_frame()?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&frame.data);
        Ok(serde_json::json!({
            "success": true,
            "data": b64,
            "width": frame.width,
            "height": frame.height,
        }))
    })();

    match result {
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
    }
}

// ─── Frame Encoding / Decoding ─────────────────────────────────────

/// Encode raw RGBA frame data for UDP transmission.
/// encoding: 0=Raw, 1=Jpeg, 2=Lz4, 3=H264
pub fn iris_encode_frame(
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
    encoding_type: u8,
) -> String {
    let encoding = match encoding_type {
        0 => FrameEncoding::RawRgba,
        1 => FrameEncoding::Jpeg,
        2 => FrameEncoding::Lz4,
        3 => FrameEncoding::H264,
        _ => FrameEncoding::Jpeg,
    };

    let frame = CapturedFrame {
        width,
        height,
        data: rgba_data,
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
            result.to_string()
        }
        Err(e) => {
            serde_json::json!({"success": false, "error": e.to_string()}).to_string()
        }
    }
}

/// Decode received frame data for display.
pub fn iris_decode_frame(
    base64_data: String,
    width: u32,
    height: u32,
    encoding_type: u8,
) -> String {
    let encoding = match encoding_type {
        0 => FrameEncoding::RawRgba,
        1 => FrameEncoding::Jpeg,
        2 => FrameEncoding::Lz4,
        3 => FrameEncoding::H264,
        _ => FrameEncoding::Jpeg,
    };

    match base64::engine::general_purpose::STANDARD.decode(&base64_data) {
        Ok(data) => match encoder::decode_frame(&data, encoding, width, height) {
            Ok(rgba) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
                let result = serde_json::json!({
                    "success": true,
                    "data": b64,
                    "width": width,
                    "height": height,
                });
                result.to_string()
            }
            Err(e) => {
                serde_json::json!({"success": false, "error": e.to_string()}).to_string()
            }
        },
        Err(e) => {
            serde_json::json!({"success": false, "error": e.to_string()}).to_string()
        }
    }
}

// ─── UDP Send / Receive ────────────────────────────────────────────

/// Send encoded frame data to a target via UDP.
pub fn iris_udp_send(target: String, encoded_b64: String) -> String {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let addr: std::net::SocketAddr = target.parse()?;
        let data = base64::engine::general_purpose::STANDARD.decode(&encoded_b64)?;

        let guard = ENGINE.lock().unwrap();
        let udp = guard
            .as_ref()
            .and_then(|e| e.udp.clone())
            .ok_or_else(|| anyhow::anyhow!("UDP not started"))?;
        drop(guard);

        let (frame_id, chunks) = block_on(async move {
            udp.send_frame(addr, 0, 1920, 1080, FrameEncoding::Jpeg, &data).await
        })?;

        Ok(serde_json::json!({"success": true, "frame_id": frame_id, "chunks": chunks}))
    })();

    match result {
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
    }
}

/// Get the latest received UDP frame (base64 RGBA + dimensions).
pub fn iris_udp_receive_latest() -> String {
    let guard = LAST_UDP_FRAME.lock().unwrap();
    match guard.as_ref() {
        Some((data, width, height)) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(data);
            serde_json::json!({"data": b64, "width": width, "height": height}).to_string()
        }
        None => serde_json::json!({"data": serde_json::Value::Null}).to_string(),
    }
}

// ─── Version ───────────────────────────────────────────────────────

/// Get the IRIS protocol version info.
pub fn iris_get_version() -> String {
    let version = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": iris_core::protocol::PROTOCOL_VERSION,
        "udp_port": iris_core::protocol::UDP_PORT,
        "tcp_port": iris_core::protocol::TCP_PORT,
    });
    version.to_string()
}

// ─── File Transfer ─────────────────────────────────────────────────

/// List shareable files from a directory (JSON).
pub fn iris_list_files(dir_path: String) -> String {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let entries: Vec<serde_json::Value> = std::fs::read_dir(&dir_path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| {
                let path = e.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                serde_json::json!({"name": name, "size": size})
            })
            .collect();
        Ok(serde_json::json!({"success": true, "files": entries}))
    })();

    match result {
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
    }
}

// ─── Display Info ──────────────────────────────────────────────────

/// Get the primary display dimensions.
pub fn iris_display_dimensions() -> String {
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let capturer = create_capture()?;
        let (w, h) = capturer.display_dimensions();
        Ok(serde_json::json!({"success": true, "width": w, "height": h}))
    })();

    match result {
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
    }
}
