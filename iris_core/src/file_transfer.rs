//! File Transfer - TCP-based reliable file sharing.
//!
//! Supports sending and receiving files over the TCP control channel.
//! Files are split into chunks and transferred with progress tracking.

use crate::protocol::{FileInfo, TcpMessage};
use crate::transport::tcp::TcpConnection;
use anyhow::Result;
use log::{debug, info};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Default chunk size for file transfer (64 KB)
const DEFAULT_CHUNK_SIZE: u32 = 64 * 1024;

/// Tracks an in-progress file transfer
struct TransferState {
    #[allow(dead_code)]
    file_id: u64,
    file_name: String,
    total_size: u64,
    received: u64,
    file: tokio::fs::File,
}

/// Manages incoming and outgoing file transfers
pub struct FileTransferManager {
    /// Directory where received files are stored
    download_dir: PathBuf,
    /// Active incoming transfers
    incoming: Arc<Mutex<HashMap<u64, TransferState>>>,
    /// Next file ID for outgoing transfers
    next_file_id: Arc<Mutex<u64>>,
}

impl FileTransferManager {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            download_dir: download_dir.into(),
            incoming: Arc::new(Mutex::new(HashMap::new())),
            next_file_id: Arc::new(Mutex::new(1)),
        }
    }

    /// List files in a directory that can be shared
    pub async fn list_shareable_files(&self, dir: &Path) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(dir).await?;
        let mut file_id = self.next_file_id.lock().await;

        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let id = *file_id;
                *file_id += 1;

                files.push(FileInfo {
                    file_id: id,
                    name,
                    size: metadata.len(),
                    mime_type: mime_guess_from_ext(&entry.path()),
                });
            }
        }

        Ok(files)
    }

    /// Send a file to a connected peer
    pub async fn send_file(
        &self,
        conn: &mut TcpConnection,
        file_path: &Path,
        file_id: u64,
    ) -> Result<()> {
        let metadata = fs::metadata(file_path).await?;
        let file_size = metadata.len();
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        info!("Sending file: {} ({} bytes)", file_name, file_size);

        // Read entire file (for LAN, this is acceptable)
        let data = fs::read(file_path).await?;

        let chunk_size = DEFAULT_CHUNK_SIZE as usize;
        let _total_chunks = ((data.len() + chunk_size - 1) / chunk_size) as u32;
        let mut offset = 0u64;

        for chunk_data in data.chunks(chunk_size) {
            let msg = TcpMessage::FileChunk {
                file_id,
                offset,
                data: chunk_data.to_vec(),
            };
            conn.send(&msg).await?;
            offset += chunk_data.len() as u64;
        }

        // Signal completion
        conn.send(&TcpMessage::FileTransferComplete { file_id })
            .await?;

        info!("File sent: {} ({} bytes)", file_name, file_size);
        Ok(())
    }

    /// Handle an incoming file chunk
    pub async fn handle_file_chunk(
        &self,
        file_id: u64,
        _offset: u64,
        data: Vec<u8>,
        file_name: &str,
        total_size: u64,
    ) -> Result<()> {
        let mut incoming = self.incoming.lock().await;

        if !incoming.contains_key(&file_id) {
            // Create new transfer state
            let file_path = self.download_dir.join(file_name);
            let file = fs::File::create(&file_path).await?;

            incoming.insert(
                file_id,
                TransferState {
                    file_id,
                    file_name: file_name.to_string(),
                    total_size,
                    received: 0,
                    file,
                },
            );

            info!("Receiving file: {} ({} bytes)", file_name, total_size);
        }

        let state = incoming.get_mut(&file_id).unwrap();
        state
            .file
            .write_all(&data)
            .await?;
        state.received += data.len() as u64;

        debug!(
            "File {} progress: {}/{} bytes",
            file_id, state.received, state.total_size
        );

        Ok(())
    }

    /// Mark a file transfer as complete
    pub async fn complete_transfer(&self, file_id: u64) -> Result<()> {
        let mut incoming = self.incoming.lock().await;
        if let Some(state) = incoming.remove(&file_id) {
            state.file.sync_all().await?;
            info!(
                "File transfer complete: {} ({} bytes)",
                state.file_name, state.total_size
            );
        }
        Ok(())
    }
}

/// Guess MIME type from file extension
fn mime_guess_from_ext(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("pdf") => "application/pdf".into(),
        Some("zip") => "application/zip".into(),
        Some("png") => "image/png".into(),
        Some("jpg") | Some("jpeg") => "image/jpeg".into(),
        Some("mp4") => "video/mp4".into(),
        Some("txt") => "text/plain".into(),
        Some("doc") | Some("docx") => "application/msword".into(),
        Some("xls") | Some("xlsx") => "application/vnd.ms-excel".into(),
        _ => "application/octet-stream".into(),
    }
}
