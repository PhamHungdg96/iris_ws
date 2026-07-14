//! IRIS Core - LAN Screen & File Sharing Engine
//!
//! Architecture:
//!   UDP (port 21000): Low-latency screen frame streaming
//!   TCP (port 21001): Reliable control messages + file transfer
//!   mDNS: LAN device discovery
//!
//! Integration: C-compatible FFI for Flutter dart:ffi

pub mod discovery;
pub mod file_transfer;
pub mod ffi;
pub mod protocol;
pub mod screen;
pub mod transport;

// Re-export key types
pub use protocol::{TcpMessage, UdpMessage, FrameEncoding, UDP_PORT, TCP_PORT};
pub use transport::{TcpTransport, UdpTransport};
