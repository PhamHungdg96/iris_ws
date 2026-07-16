//! TCP Transport - Reliable control messages and file transfer.
//!
//! Each connection is handled by a TcpConnection that manages
//! JSON-framed message serialization and deserialization.

use crate::protocol::{TcpMessage, TCP_PORT};
use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// How many ports to try if the default is in use
const MAX_PORT_RETRIES: u16 = 100;

/// Manages TCP connections for control + file transfer
pub struct TcpTransport {
    listener: TcpListener,
    local_addr: SocketAddr,
}

/// Handle to an established TCP connection
pub struct TcpConnection {
    stream: BufReader<TcpStream>,
    peer_addr: SocketAddr,
}

impl TcpTransport {
    /// Create and bind TCP listener on the default port.
    /// Falls back to next available port if default is in use.
    pub async fn bind() -> Result<Self> {
        Self::bind_from(TCP_PORT).await
    }

    /// Create and bind TCP listener starting from a specific port.
    /// Auto-increments if the port is already in use.
    pub async fn bind_from(start_port: u16) -> Result<Self> {
        for offset in 0..MAX_PORT_RETRIES {
            let port = start_port + offset;
            let addr = format!("0.0.0.0:{}", port);

            match TcpListener::bind(&addr).await {
                Ok(listener) => {
                    let local_addr = listener.local_addr()?;
                    info!("TCP transport listening on {}", local_addr);
                    return Ok(Self { listener, local_addr });
                }
                Err(e) if offset == 0 => {
                    debug!("TCP port {} in use, trying next... ({})", port, e);
                }
                Err(_) => {
                    // Continue trying next port
                }
            }
        }
        anyhow::bail!(
            "Failed to bind TCP: all ports {}-{} are in use",
            start_port,
            start_port + MAX_PORT_RETRIES - 1
        )
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Accept incoming connections, returning a channel of (connection, messages).
    /// Each accepted connection spawns a task that reads messages.
    pub async fn accept_loop(
        &self,
        msg_tx: mpsc::UnboundedSender<(TcpMessage, SocketAddr)>,
    ) -> Result<()> {
        loop {
            let (stream, addr) = self.listener.accept().await?;
            info!("TCP connection accepted from {}", addr);

            let tx = msg_tx.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, tx).await {
                    error!("Connection error from {}: {}", addr, e);
                }
            });
        }
    }

    /// Connect to a remote device
    pub async fn connect(addr: SocketAddr) -> Result<TcpConnection> {
        let stream = TcpStream::connect(addr)
            .await
            .context(format!("Failed to connect to {}", addr))?;
        info!("TCP connected to {}", addr);
        Ok(TcpConnection {
            stream: BufReader::new(stream),
            peer_addr: addr,
        })
    }
}

impl TcpConnection {
    /// Send a TCP message with length-prefixed framing
    pub async fn send(&mut self, msg: &TcpMessage) -> Result<()> {
        let data = msg.to_bytes();
        self.stream.get_mut().write_all(&data).await?;
        Ok(())
    }

    /// Receive a TCP message
    pub async fn recv(&mut self) -> Result<TcpMessage> {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Sanity check
        if len > 100 * 1024 * 1024 {
            // 100 MB max
            anyhow::bail!("Message too large: {} bytes", len);
        }

        // Read payload
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload).await?;

        serde_json::from_slice(&payload).context("Failed to decode TCP message")
    }

    /// Send a file chunk (used by file transfer)
    pub async fn send_raw(&mut self, data: &[u8]) -> Result<()> {
        self.stream.get_mut().write_all(data).await?;
        Ok(())
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}

/// Handle a single TCP connection, reading messages into the channel
async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    tx: mpsc::UnboundedSender<(TcpMessage, SocketAddr)>,
) -> Result<()> {
    loop {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            // Connection closed
            break;
        }
        let len = u32::from_be_bytes(len_buf) as usize;

        if len > 100 * 1024 * 1024 {
            warn!("Oversized message from {}: {} bytes", addr, len);
            break;
        }

        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).await?;

        match serde_json::from_slice::<TcpMessage>(&payload) {
            Ok(msg) => {
                if tx.send((msg, addr)).is_err() {
                    break; // Receiver dropped
                }
            }
            Err(e) => {
                warn!("Failed to decode message from {}: {}", addr, e);
            }
        }
    }

    debug!("Connection closed: {}", addr);
    Ok(())
}
