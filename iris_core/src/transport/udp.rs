//! UDP Transport - Low-latency screen frame streaming.
//!
//! Uses tokio UdpSocket for async I/O. Splits frames into chunks
//! to stay under the typical MTU (~1400 bytes payload per packet).

use crate::protocol::{FrameChunk, FrameEncoding, UdpMessage, UDP_PORT};
use anyhow::{Context, Result};
use log::{debug, info, trace};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

/// Maximum payload per UDP packet (leave room for headers)
const MAX_CHUNK_SIZE: usize = 1300;

/// Socket buffer size for streaming (8 MB)
const SOCKET_BUF_SIZE: usize = 8 * 1024 * 1024;

/// How many ports to try if the default is in use
const MAX_PORT_RETRIES: u16 = 100;

pub struct UdpTransport {
    socket: UdpSocket,
    local_addr: SocketAddr,
}

impl UdpTransport {
    /// Create a new UDP transport, starting from the default port.
    /// Falls back to next available port if default is in use.
    pub async fn bind() -> Result<Self> {
        Self::bind_from(UDP_PORT).await
    }

    /// Create a new UDP transport starting from a specific port.
    /// Auto-increments if the port is already in use.
    pub async fn bind_from(start_port: u16) -> Result<Self> {
        for offset in 0..MAX_PORT_RETRIES {
            let port = start_port + offset;
            let addr: std::net::SocketAddr = format!("0.0.0.0:{}", port).parse()?;

            match Self::try_bind_addr(addr) {
                Ok(transport) => {
                    info!("UDP transport bound to port {}", port);
                    return Ok(transport);
                }
                Err(e) if offset == 0 => {
                    debug!("UDP port {} in use, trying next... ({})", port, e);
                }
                Err(_) => {
                    // Continue trying next port
                }
            }
        }
        anyhow::bail!("Failed to bind UDP: all ports {}-{} are in use", start_port, start_port + MAX_PORT_RETRIES - 1)
    }

    fn try_bind_addr(addr: std::net::SocketAddr) -> Result<Self> {
        let socket2 = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;

        socket2.set_reuse_address(true)?;
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        socket2.set_reuse_port(true)?;

        if let Err(e) = socket2.set_recv_buffer_size(SOCKET_BUF_SIZE) {
            debug!("Could not set recv buffer size: {}", e);
        }
        if let Err(e) = socket2.set_send_buffer_size(SOCKET_BUF_SIZE) {
            debug!("Could not set send buffer size: {}", e);
        }

        socket2.bind(&socket2::SockAddr::from(addr))?;
        socket2.set_nonblocking(true)?;

        let std_socket: std::net::UdpSocket = socket2.into();
        let socket = UdpSocket::from_std(std_socket)?;
        let local_addr = socket.local_addr()?;

        Ok(Self { socket, local_addr })
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send a full encoded frame, splitting into chunks.
    /// Returns the frame_id and number of chunks sent.
    pub async fn send_frame(
        &self,
        target: SocketAddr,
        frame_id: u64,
        width: u32,
        height: u32,
        encoding: FrameEncoding,
        data: &[u8],
    ) -> Result<(u64, u32)> {
        let total_chunks =
            ((data.len() + MAX_CHUNK_SIZE - 1) / MAX_CHUNK_SIZE) as u32;

        for (i, chunk_data) in data.chunks(MAX_CHUNK_SIZE).enumerate() {
            let msg = UdpMessage::FrameChunk(FrameChunk {
                frame_id,
                total_chunks,
                chunk_index: i as u32,
                width,
                height,
                encoding,
                data: chunk_data.to_vec(),
            });

            let packet = msg.to_bytes();
            self.socket.send_to(&packet, target).await?;
        }

        trace!(
            "Sent frame {} ({} chunks, {} bytes) to {}",
            frame_id,
            total_chunks,
            data.len(),
            target
        );

        Ok((frame_id, total_chunks))
    }

    /// Request a keyframe from a sender (for new viewers)
    pub async fn request_keyframe(&self, target: SocketAddr) -> Result<()> {
        let msg = UdpMessage::KeyFrameRequest;
        self.socket.send_to(&msg.to_bytes(), target).await?;
        Ok(())
    }

    /// Send stream control command
    pub async fn send_control(
        &self,
        target: SocketAddr,
        msg: UdpMessage,
    ) -> Result<()> {
        self.socket.send_to(&msg.to_bytes(), target).await?;
        Ok(())
    }

    /// Receive a UDP message.
    pub async fn recv(&self) -> Result<(UdpMessage, SocketAddr)> {
        let mut buf = vec![0u8; 65535];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;

        UdpMessage::from_bytes(&buf[..len])
            .map(|msg| (msg, addr))
            .context("Failed to decode UDP message")
    }

    /// Get the raw socket (for select/poll integration)
    pub fn socket_ref(&self) -> &UdpSocket {
        &self.socket
    }
}

/// Reassemble frame chunks into a complete frame.
pub struct FrameAssembler {
    frame_id: u64,
    total_chunks: u32,
    chunks: Vec<Option<Vec<u8>>>,
    width: u32,
    height: u32,
    encoding: FrameEncoding,
    received: u32,
}

impl FrameAssembler {
    pub fn new(chunk: &FrameChunk) -> Self {
        let mut chunks = Vec::with_capacity(chunk.total_chunks as usize);
        chunks.resize_with(chunk.total_chunks as usize, || None);

        let mut assembler = Self {
            frame_id: chunk.frame_id,
            total_chunks: chunk.total_chunks,
            chunks,
            width: chunk.width,
            height: chunk.height,
            encoding: chunk.encoding,
            received: 0,
        };
        assembler.add_chunk(chunk);
        assembler
    }

    pub fn add_chunk(&mut self, chunk: &FrameChunk) {
        if chunk.frame_id != self.frame_id {
            return; // Different frame, ignore
        }
        let idx = chunk.chunk_index as usize;
        if idx < self.chunks.len() && self.chunks[idx].is_none() {
            self.chunks[idx] = Some(chunk.data.clone());
            self.received += 1;
        }
    }

    pub fn is_complete(&self) -> bool {
        self.received >= self.total_chunks
    }

    pub fn assemble(self) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }
        let total_size: usize = self.chunks.iter().filter_map(|c| c.as_ref()).map(|c| c.len()).sum();
        let mut data = Vec::with_capacity(total_size);
        for chunk in self.chunks {
            if let Some(c) = chunk {
                data.extend_from_slice(&c);
            }
        }
        Some(data)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn encoding(&self) -> FrameEncoding {
        self.encoding
    }

    pub fn frame_id(&self) -> u64 {
        self.frame_id
    }
}
