//! IRIS Protocol - Message definitions for UDP streaming + TCP control/file transfer.
//!
//! Architecture:
//!   UDP (port 21000): Screen frame chunks, low-latency, loss-tolerant
//!   TCP (port 21001): Control commands, file transfer, reliable delivery

use serde::{Deserialize, Serialize};

/// Magic bytes to identify IRIS protocol packets
pub const MAGIC: [u8; 4] = *b"IRIS";
pub const PROTOCOL_VERSION: u16 = 1;

/// Default ports
pub const UDP_PORT: u16 = 21000;
pub const TCP_PORT: u16 = 21001;

/// Service type for mDNS discovery
pub const MDNS_SERVICE_TYPE: &str = "_iris._udp.local.";
pub const MDNS_SERVICE_NAME: &str = "IRIS Screen Share";

// ─── UDP Messages (Screen Streaming) ───────────────────────────────

/// UDP packet types for screen streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UdpMessage {
    /// A chunk of an encoded screen frame
    FrameChunk(FrameChunk),
    /// Request a keyframe (for new viewers joining mid-stream)
    KeyFrameRequest,
    /// Acknowledge received chunks (lightweight flow control)
    Ack { frame_id: u64, chunk_count: u32 },
    /// Stream control: pause/resume
    StreamControl { action: StreamAction },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameChunk {
    /// Monotonically increasing frame ID
    pub frame_id: u64,
    /// Total chunks in this frame
    pub total_chunks: u32,
    /// Index of this chunk (0-based)
    pub chunk_index: u32,
    /// Screen dimensions
    pub width: u32,
    pub height: u32,
    /// Encoding type
    pub encoding: FrameEncoding,
    /// Raw encoded chunk data
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameEncoding {
    /// Raw RGBA pixels (no compression, for LAN)
    RawRgba = 0,
    /// JPEG compressed frame
    Jpeg = 1,
    /// LZ4 compressed raw pixels
    Lz4 = 2,
    /// H.264 software encoding (via openh264)
    H264 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamAction {
    Pause,
    Resume,
    Stop,
}

// ─── TCP Messages (Control + File Transfer) ────────────────────────

/// TCP message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TcpMessage {
    // ── Handshake ──
    Hello(HelloInfo),
    HelloAck { session_id: u64 },

    // ── Discovery ──
    DeviceAnnounce(DeviceInfo),
    DeviceQuery,
    DeviceList(Vec<DeviceInfo>),

    // ── Screen Share Control ──
    StartScreenShare { quality: u8, max_fps: u8 },
    StopScreenShare,
    ScreenShareStarted { udp_port: u16, session_id: u64 },

    // ── File Transfer ──
    FileListRequest,
    FileListResponse(Vec<FileInfo>),
    FileTransferRequest { file_id: u64 },
    FileTransferAccept { file_id: u64, chunk_size: u32 },
    FileTransferReject { file_id: u64, reason: String },
    FileChunk {
        file_id: u64,
        offset: u64,
        data: Vec<u8>,
    },
    FileTransferComplete { file_id: u64 },

    // ── General ──
    Error { code: u32, message: String },
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloInfo {
    pub device_name: String,
    pub platform: String,
    pub version: String,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub screen_share: bool,
    pub file_transfer: bool,
    pub max_resolution: (u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub ip_addresses: Vec<String>,
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub file_id: u64,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
}

// ─── Packet Serialization Helpers ──────────────────────────────────

impl UdpMessage {
    /// Serialize to bytes for UDP transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1024);
        buf.extend_from_slice(&MAGIC);
        buf.push(0); // message type: 0 = UDP
        buf.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        // Use bincode for compact serialization
        let payload = bincode::serialize(self).unwrap_or_default();
        buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&payload);
        buf
    }

    /// Deserialize from UDP packet bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 11 {
            return None;
        }
        if data[0..4] != MAGIC {
            return None;
        }
        let payload_len = u32::from_be_bytes([data[7], data[8], data[9], data[10]]) as usize;
        let payload = data.get(11..11 + payload_len)?;
        bincode::deserialize(payload).ok()
    }
}

impl TcpMessage {
    /// Serialize with length prefix for TCP framing
    pub fn to_bytes(&self) -> Vec<u8> {
        let payload = serde_json::to_vec(self).unwrap_or_default();
        let mut buf = Vec::with_capacity(4 + payload.len());
        buf.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&payload);
        buf
    }
}
