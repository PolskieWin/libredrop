use std::io::{Error, ErrorKind, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const MAGIC_BYTES: &[u8; 4] = b"LDRP";
pub const PROTOCOL_VERSION: u8 = 1;

pub const FRAME_HANDSHAKE_REQ: u8 = 0x01;
pub const FRAME_HANDSHAKE_RESP: u8 = 0x02;
pub const FRAME_FILE_START: u8 = 0x10;
pub const FRAME_FILE_CHUNK: u8 = 0x11;
pub const FRAME_FILE_END: u8 = 0x12;
pub const FRAME_FILE_ACK: u8 = 0x13;
pub const FRAME_TRANSFER_COMPLETE: u8 = 0x20;
pub const FRAME_TEXT_MESSAGE: u8 = 0x30;
pub const FRAME_ERROR: u8 = 0xFF;

pub const MAX_FRAME_PAYLOAD_SIZE: usize = 16 * 1024 * 1024; // 16 MB max frame size
pub const OPTIMAL_CHUNK_SIZE: usize = 512 * 1024; // 512 KB per file chunk for max LAN throughput

#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new(frame_type: u8, payload: Vec<u8>) -> Self {
        Self {
            frame_type,
            payload,
        }
    }

    pub fn text_message(text: &str) -> Self {
        Self {
            frame_type: FRAME_TEXT_MESSAGE,
            payload: text.as_bytes().to_vec(),
        }
    }

    pub fn file_ack(file_id: &str, success: bool) -> Self {
        let status = if success { 1u8 } else { 0u8 };
        let mut payload = Vec::with_capacity(file_id.len() + 1);
        payload.push(status);
        payload.extend_from_slice(file_id.as_bytes());
        Self {
            frame_type: FRAME_FILE_ACK,
            payload,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            frame_type: FRAME_ERROR,
            payload: msg.as_bytes().to_vec(),
        }
    }
}

pub async fn write_frame<W>(writer: &mut W, frame_type: u8, payload: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() > MAX_FRAME_PAYLOAD_SIZE {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Frame payload exceeds maximum allowed size ({} bytes)", MAX_FRAME_PAYLOAD_SIZE),
        ));
    }

    let mut header = [0u8; 10];
    header[0..4].copy_from_slice(MAGIC_BYTES);
    header[4] = PROTOCOL_VERSION;
    header[5] = frame_type;
    let len = payload.len() as u32;
    header[6..10].copy_from_slice(&len.to_be_bytes());

    writer.write_all(&header).await?;
    if !payload.is_empty() {
        writer.write_all(payload).await?;
    }
    writer.flush().await?;
    Ok(())
}

pub async fn read_frame<R>(reader: &mut R) -> Result<Frame>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0u8; 10];
    reader.read_exact(&mut header).await?;

    if &header[0..4] != MAGIC_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid LibreDrop magic header",
        ));
    }

    let version = header[4];
    if version != PROTOCOL_VERSION {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Unsupported LibreDrop protocol version: {}", version),
        ));
    }

    let frame_type = header[5];
    let payload_len = u32::from_be_bytes([header[6], header[7], header[8], header[9]]) as usize;

    if payload_len > MAX_FRAME_PAYLOAD_SIZE {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Frame length {} exceeds maximum allowed {}", payload_len, MAX_FRAME_PAYLOAD_SIZE),
        ));
    }

    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        reader.read_exact(&mut payload).await?;
    }

    Ok(Frame {
        frame_type,
        payload,
    })
}
