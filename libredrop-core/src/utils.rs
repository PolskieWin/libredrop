use sha2::{Digest, Sha256};
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn get_local_ip() -> IpAddr {
    // Determine the local IP by checking the outbound route via an unconnected UDP socket
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip();
            }
        }
    }
    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
}

pub fn compute_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

pub fn get_default_download_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let path = std::path::PathBuf::from(userprofile).join("Downloads");
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::PathBuf::from(home).join("Downloads");
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
    }
    "./Downloads".to_string()
}

pub fn get_default_device_model() -> String {
    #[cfg(target_os = "windows")]
    {
        "Windows PC".to_string()
    }
    #[cfg(target_os = "android")]
    {
        "Android Device".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "macOS Device".to_string()
    }
    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    {
        "Linux PC".to_string()
    }
    #[cfg(not(any(target_os = "windows", target_os = "android", target_os = "macos", target_os = "linux")))]
    {
        "LibreDrop Node".to_string()
    }
}
