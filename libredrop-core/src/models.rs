use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Mobile,
    Desktop,
    Tablet,
    Server,
    Cli,
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Mobile
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub alias: String,
    pub device_model: String,
    pub device_type: DeviceType,
    pub address: String,
    pub port: u16,
    pub last_seen_epoch_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub transfer_id: String,
    pub sender_alias: String,
    pub sender_model: String,
    pub transfer_type: String, // "message" or "files"
    pub message: Option<String>,
    pub files: Vec<TransferItem>,
    pub total_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponse {
    pub transfer_id: String,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub transfer_id: String,
    pub current_file_index: usize,
    pub total_files: usize,
    pub current_file_name: String,
    pub current_file_bytes: u64,
    pub current_file_total: u64,
    pub total_bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bytes_sec: u64,
    pub progress_ratio: f32,
    pub is_completed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropConfig {
    pub device_id: String,
    pub alias: String,
    pub device_model: String,
    pub device_type: DeviceType,
    pub port: u16,
    pub discovery_port: u16,
    pub download_dir: String,
    pub auto_accept: bool,
}

impl Default for DropConfig {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        let dev_type = DeviceType::Desktop;
        #[cfg(not(target_os = "windows"))]
        let dev_type = DeviceType::Mobile;

        Self {
            device_id: "ld-node".to_string(),
            alias: crate::utils::get_default_device_model(),
            device_model: crate::utils::get_default_device_model(),
            device_type: dev_type,
            port: 53317,
            discovery_port: 53318,
            download_dir: crate::utils::get_default_download_dir(),
            auto_accept: false,
        }
    }
}
