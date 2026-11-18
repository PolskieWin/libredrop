use crate::models::{DropConfig, TransferItem, TransferProgress, TransferRequest, TransferResponse};
use crate::protocol::*;
use crate::utils::get_epoch_ms;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::{create_dir_all, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::time::{sleep, Duration, Instant};

pub type RequestCallback = Arc<dyn Fn(TransferRequest) -> bool + Send + Sync>;
pub type MessageCallback = Arc<dyn Fn(String, String) -> () + Send + Sync>; // sender_alias, text

pub struct TransferManager {
    config: Arc<RwLock<DropConfig>>,
    is_running: Arc<AtomicBool>,
    request_handler: Arc<Mutex<Option<RequestCallback>>>,
    message_handler: Arc<Mutex<Option<MessageCallback>>>,
    progress_tx: broadcast::Sender<TransferProgress>,
}

impl TransferManager {
    pub fn new(config: DropConfig) -> Self {
        let (tx, _) = broadcast::channel(128);
        Self {
            config: Arc::new(RwLock::new(config)),
            is_running: Arc::new(AtomicBool::new(false)),
            request_handler: Arc::new(Mutex::new(None)),
            message_handler: Arc::new(Mutex::new(None)),
            progress_tx: tx,
        }
    }

    pub fn subscribe_progress(&self) -> broadcast::Receiver<TransferProgress> {
        self.progress_tx.subscribe()
    }

    pub async fn set_request_handler(&self, handler: RequestCallback) {
        let mut lock = self.request_handler.lock().await;
        *lock = Some(handler);
    }

    pub async fn set_message_handler(&self, handler: MessageCallback) {
        let mut lock = self.message_handler.lock().await;
        *lock = Some(handler);
    }

    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.is_running.store(true, Ordering::SeqCst);

        let port = self.config.read().await.port;
        let bind_addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&bind_addr).await?;

        let is_running = self.is_running.clone();
        let config = self.config.clone();
        let req_handler = self.request_handler.clone();
        let msg_handler = self.message_handler.clone();
        let progress_tx = self.progress_tx.clone();

        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                tokio::select! {
                    accept_res = listener.accept() => {
                        match accept_res {
                            Ok((stream, peer_addr)) => {
                                let conf = config.clone();
                                let rh = req_handler.clone();
                                let mh = msg_handler.clone();
                                let ptx = progress_tx.clone();

                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_incoming_connection(stream, conf, rh, mh, ptx).await {
                                        eprintln!("Error handling connection from {}: {}", peer_addr, e);
                                    }
                                });
                            }
                            Err(e) => {
                                eprintln!("Accept error: {}", e);
                                sleep(Duration::from_millis(200)).await;
                            }
                        }
                    }
                    _ = sleep(Duration::from_millis(500)) => {}
                }
            }
        });

        Ok(())
    }

    pub fn stop_server(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    async fn handle_incoming_connection(
        mut stream: TcpStream,
        config: Arc<RwLock<DropConfig>>,
        req_handler: Arc<Mutex<Option<RequestCallback>>>,
        msg_handler: Arc<Mutex<Option<MessageCallback>>>,
        progress_tx: broadcast::Sender<TransferProgress>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _ = stream.set_nodelay(true);

        // 1. Read Handshake Request
        let frame = read_frame(&mut stream).await?;
        if frame.frame_type != FRAME_HANDSHAKE_REQ {
            return Err("Expected handshake request frame".into());
        }

        let request: TransferRequest = serde_json::from_slice(&frame.payload)?;
        let transfer_id = request.transfer_id.clone();
        let sender_alias = request.sender_alias.clone();
        let is_message = request.transfer_type == "message";

        // Check acceptance
        let auto_accept = config.read().await.auto_accept;
        let mut accepted = auto_accept;

        if !accepted {
            let handler_lock = req_handler.lock().await;
            if let Some(ref cb) = *handler_lock {
                accepted = cb(request.clone());
            } else {
                accepted = true; // default accept if no interactive handler registered
            }
        }

        let resp = TransferResponse {
            transfer_id: transfer_id.clone(),
            accepted,
            reason: if accepted { None } else { Some("Transfer declined by recipient".to_string()) },
        };

        let resp_payload = serde_json::to_vec(&resp)?;
        write_frame(&mut stream, FRAME_HANDSHAKE_RESP, &resp_payload).await?;

        if !accepted {
            return Ok(());
        }

        // Handle text message drop
        if is_message {
            if let Some(msg) = request.message {
                let handler_lock = msg_handler.lock().await;
                if let Some(ref cb) = *handler_lock {
                    cb(sender_alias.clone(), msg.clone());
                }
            } else {
                // Read following text frame
                let text_frame = read_frame(&mut stream).await?;
                if text_frame.frame_type == FRAME_TEXT_MESSAGE {
                    let text = String::from_utf8_lossy(&text_frame.payload).to_string();
                    let handler_lock = msg_handler.lock().await;
                    if let Some(ref cb) = *handler_lock {
                        cb(sender_alias.clone(), text);
                    }
                }
            }
            return Ok(());
        }

        // Handle file drop
        let download_dir = config.read().await.download_dir.clone();
        create_dir_all(&download_dir).await?;

        let total_files = request.files.len();
        let total_bytes = request.total_size;
        let mut total_transferred: u64 = 0;
        let mut last_speed_calc = Instant::now();
        let mut last_bytes = 0u64;
        let mut current_speed: u64;

        for (file_idx, item) in request.files.iter().enumerate() {
            // Wait for FRAME_FILE_START
            let frame = read_frame(&mut stream).await?;
            if frame.frame_type != FRAME_FILE_START {
                return Err(format!("Expected FILE_START frame for file {}", item.name).into());
            }

            // Sanitize file name to prevent path traversal
            let sanitized_name = Path::new(&item.name)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("drop_file_{}", file_idx));

            let dest_path = PathBuf::from(&download_dir).join(&sanitized_name);
            let mut file = File::create(&dest_path).await?;
            let mut hasher = Sha256::new();
            let mut file_bytes_transferred: u64 = 0;

            loop {
                let chunk_frame = read_frame(&mut stream).await?;
                match chunk_frame.frame_type {
                    FRAME_FILE_CHUNK => {
                        let chunk = &chunk_frame.payload;
                        file.write_all(chunk).await?;
                        hasher.update(chunk);

                        let chunk_len = chunk.len() as u64;
                        file_bytes_transferred += chunk_len;
                        total_transferred += chunk_len;

                        if last_speed_calc.elapsed() >= Duration::from_millis(500) {
                            let elapsed_secs = last_speed_calc.elapsed().as_secs_f64();
                            let bytes_delta = total_transferred.saturating_sub(last_bytes);
                            current_speed = (bytes_delta as f64 / elapsed_secs) as u64;
                            last_speed_calc = Instant::now();
                            last_bytes = total_transferred;

                            let ratio = if total_bytes > 0 {
                                (total_transferred as f32 / total_bytes as f32).clamp(0.0, 1.0)
                            } else {
                                1.0
                            };

                            let _ = progress_tx.send(TransferProgress {
                                transfer_id: transfer_id.clone(),
                                current_file_index: file_idx,
                                total_files,
                                current_file_name: sanitized_name.clone(),
                                current_file_bytes: file_bytes_transferred,
                                current_file_total: item.size,
                                total_bytes_transferred: total_transferred,
                                total_bytes,
                                speed_bytes_sec: current_speed,
                                progress_ratio: ratio,
                                is_completed: false,
                                error: None,
                            });
                        }
                    }
                    FRAME_FILE_END => {
                        file.flush().await?;
                        let calculated_hash = format!("{:x}", hasher.finalize());
                        let remote_hash = String::from_utf8_lossy(&chunk_frame.payload).to_string();

                        let is_valid = if !remote_hash.is_empty() {
                            calculated_hash == remote_hash
                        } else {
                            true
                        };

                        let ack = Frame::file_ack(&item.id, is_valid);
                        write_frame(&mut stream, ack.frame_type, &ack.payload).await?;
                        break;
                    }
                    FRAME_ERROR => {
                        let err_msg = String::from_utf8_lossy(&chunk_frame.payload);
                        return Err(format!("Sender reported error: {}", err_msg).into());
                    }
                    _ => {
                        return Err(format!("Unexpected frame type: 0x{:02X}", chunk_frame.frame_type).into());
                    }
                }
            }
        }

        // Read TRANSFER_COMPLETE
        let final_frame = read_frame(&mut stream).await?;
        if final_frame.frame_type == FRAME_TRANSFER_COMPLETE {
            let _ = progress_tx.send(TransferProgress {
                transfer_id: transfer_id.clone(),
                current_file_index: total_files.saturating_sub(1),
                total_files,
                current_file_name: "Completed".to_string(),
                current_file_bytes: total_transferred,
                current_file_total: total_bytes,
                total_bytes_transferred: total_transferred,
                total_bytes,
                speed_bytes_sec: 0,
                progress_ratio: 1.0,
                is_completed: true,
                error: None,
            });
        }

        Ok(())
    }

    // Client: Send files or message to target peer
    pub async fn send_transfer(
        target_addr: &str,
        target_port: u16,
        config: &DropConfig,
        message: Option<String>,
        file_paths: Vec<String>,
        progress_tx: broadcast::Sender<TransferProgress>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connect_addr = format!("{}:{}", target_addr, target_port);
        let mut stream = TcpStream::connect(&connect_addr).await?;
        let _ = stream.set_nodelay(true);

        let transfer_id = format!("tx-{}", get_epoch_ms());
        let is_message = message.is_some() && file_paths.is_empty();

        let mut items = Vec::new();
        let mut total_size = 0u64;

        if !is_message {
            for (idx, path_str) in file_paths.iter().enumerate() {
                let path = Path::new(path_str);
                let metadata = tokio::fs::metadata(path).await?;
                let file_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("file_{}", idx));
                let size = metadata.len();
                total_size += size;

                let mime_type = match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                    "jpg" | "jpeg" => "image/jpeg",
                    "png" => "image/png",
                    "gif" => "image/gif",
                    "mp4" => "video/mp4",
                    "mkv" => "video/x-matroska",
                    "zip" => "application/zip",
                    "pdf" => "application/pdf",
                    "apk" => "application/vnd.android.package-archive",
                    _ => "application/octet-stream",
                }
                .to_string();

                items.push(TransferItem {
                    id: format!("f-{}", idx),
                    name: file_name,
                    size,
                    mime_type,
                    sha256: None,
                });
            }
        }

        let req = TransferRequest {
            transfer_id: transfer_id.clone(),
            sender_alias: config.alias.clone(),
            sender_model: config.device_model.clone(),
            transfer_type: if is_message { "message".to_string() } else { "files".to_string() },
            message: message.clone(),
            files: items.clone(),
            total_size,
        };

        let req_bytes = serde_json::to_vec(&req)?;
        write_frame(&mut stream, FRAME_HANDSHAKE_REQ, &req_bytes).await?;

        // Await Handshake Response
        let resp_frame = read_frame(&mut stream).await?;
        if resp_frame.frame_type != FRAME_HANDSHAKE_RESP {
            return Err("Expected handshake response".into());
        }

        let resp: TransferResponse = serde_json::from_slice(&resp_frame.payload)?;
        if !resp.accepted {
            return Err(format!("Recipient rejected transfer: {:?}", resp.reason).into());
        }

        if is_message {
            // Text message transferred directly in handshake payload or standalone frame
            let _ = progress_tx.send(TransferProgress {
                transfer_id,
                current_file_index: 0,
                total_files: 1,
                current_file_name: "Text Message".to_string(),
                current_file_bytes: 1,
                current_file_total: 1,
                total_bytes_transferred: 1,
                total_bytes: 1,
                speed_bytes_sec: 0,
                progress_ratio: 1.0,
                is_completed: true,
                error: None,
            });
            return Ok(());
        }

        // Stream files
        let total_files = items.len();
        let mut total_transferred: u64 = 0;
        let mut last_speed_calc = Instant::now();
        let mut last_bytes = 0u64;
        let mut current_speed: u64;

        let mut buffer = vec![0u8; OPTIMAL_CHUNK_SIZE];

        for (file_idx, path_str) in file_paths.iter().enumerate() {
            let item = &items[file_idx];
            let mut file = File::open(path_str).await?;
            let mut hasher = Sha256::new();
            let mut file_transferred: u64 = 0;

            // Send FILE_START
            let start_payload = serde_json::to_vec(item)?;
            write_frame(&mut stream, FRAME_FILE_START, &start_payload).await?;

            loop {
                let n = file.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }

                let chunk = &buffer[..n];
                hasher.update(chunk);
                write_frame(&mut stream, FRAME_FILE_CHUNK, chunk).await?;

                let n_u64 = n as u64;
                file_transferred += n_u64;
                total_transferred += n_u64;

                if last_speed_calc.elapsed() >= Duration::from_millis(500) {
                    let elapsed_secs = last_speed_calc.elapsed().as_secs_f64();
                    let bytes_delta = total_transferred.saturating_sub(last_bytes);
                    current_speed = (bytes_delta as f64 / elapsed_secs) as u64;
                    last_speed_calc = Instant::now();
                    last_bytes = total_transferred;

                    let ratio = if total_size > 0 {
                        (total_transferred as f32 / total_size as f32).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };

                    let _ = progress_tx.send(TransferProgress {
                        transfer_id: transfer_id.clone(),
                        current_file_index: file_idx,
                        total_files,
                        current_file_name: item.name.clone(),
                        current_file_bytes: file_transferred,
                        current_file_total: item.size,
                        total_bytes_transferred: total_transferred,
                        total_bytes: total_size,
                        speed_bytes_sec: current_speed,
                        progress_ratio: ratio,
                        is_completed: false,
                        error: None,
                    });
                }
            }

            // Send FILE_END with SHA-256 hash
            let hash = format!("{:x}", hasher.finalize());
            write_frame(&mut stream, FRAME_FILE_END, hash.as_bytes()).await?;

            // Read ACK
            let ack_frame = read_frame(&mut stream).await?;
            if ack_frame.frame_type != FRAME_FILE_ACK {
                return Err(format!("Expected FILE_ACK for file {}", item.name).into());
            }

            if ack_frame.payload.is_empty() || ack_frame.payload[0] != 1 {
                return Err(format!("File verification failed on recipient for {}", item.name).into());
            }
        }

        // Send TRANSFER_COMPLETE
        write_frame(&mut stream, FRAME_TRANSFER_COMPLETE, &[]).await?;

        let _ = progress_tx.send(TransferProgress {
            transfer_id,
            current_file_index: total_files.saturating_sub(1),
            total_files,
            current_file_name: "Transfer Complete".to_string(),
            current_file_bytes: total_transferred,
            current_file_total: total_size,
            total_bytes_transferred: total_transferred,
            total_bytes: total_size,
            speed_bytes_sec: 0,
            progress_ratio: 1.0,
            is_completed: true,
            error: None,
        });

        Ok(())
    }
}
