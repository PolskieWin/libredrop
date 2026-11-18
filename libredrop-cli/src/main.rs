use libredrop_core::discovery::DiscoveryManager;
use libredrop_core::models::{DeviceType, DropConfig};
use libredrop_core::transfer::TransferManager;
use libredrop_core::utils::{format_bytes, format_speed, get_local_ip};
use libredrop_core::web_drop::WebDropServer;
use std::env;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let command = args[1].as_str();

    match command {
        "scan" => {
            cmd_scan().await?;
        }
        "listen" | "serve" => {
            let mut port = 53317u16;
            let mut dir = "./Downloads".to_string();
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--port" && i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(53317);
                    i += 2;
                } else if args[i] == "--dir" && i + 1 < args.len() {
                    dir = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            cmd_listen(port, dir).await?;
        }
        "send" => {
            if args.len() < 4 {
                eprintln!("Usage: libredrop send <target-ip> <file-path-1> [file-path-2...]");
                std::process::exit(1);
            }
            let target_ip = &args[2];
            let files = args[3..].to_vec();
            cmd_send_files(target_ip, 53317, files).await?;
        }
        "msg" | "message" => {
            if args.len() < 4 {
                eprintln!("Usage: libredrop msg <target-ip> <message-text>");
                std::process::exit(1);
            }
            let target_ip = &args[2];
            let message = args[3..].join(" ");
            cmd_send_message(target_ip, 53317, message).await?;
        }
        "web" => {
            let mut port = 53319u16;
            if args.len() >= 3 {
                port = args[2].parse().unwrap_or(53319);
            }
            cmd_web(port).await?;
        }
        "status" => {
            cmd_status().await;
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            eprintln!("Unknown command: {}", other);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_help() {
    println!("LibreDrop CLI - Ultra-fast, zero-downscale local drop utility");
    println!("Usage:");
    println!("  libredrop scan                      Discover nearby LibreDrop peers on local LAN");
    println!("  libredrop listen [--port N] [--dir] Run daemon to receive files and messages");
    println!("  libredrop send <ip> <file1> [file2] Send original files with zero downscaling");
    println!("  libredrop msg <ip> <text>           Send instant text note or clipboard drop");
    println!("  libredrop web [port]                Launch Web Drop server for browser uploads");
    println!("  libredrop status                    Show local IP, configuration, and ports");
}

async fn cmd_status() {
    let local_ip = get_local_ip();
    println!("LibreDrop Status:");
    println!("  Local IP Address:  {}", local_ip);
    println!("  Transfer Port:     53317 (TCP)");
    println!("  Discovery Port:    53318 (UDP)");
    println!("  Web Drop Default:  53319 (HTTP)");
    println!("  Encryption / Hash: SHA-256 Bit-for-bit");
    println!("  Compression:       0% (Lossless raw streams)");
}

async fn cmd_scan() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = DropConfig::default();
    config.alias = "LibreDrop CLI Scanner".to_string();
    config.device_type = DeviceType::Cli;

    let discovery = DiscoveryManager::new(config);
    println!("Scanning for LibreDrop peers on LAN (press Ctrl+C to stop)...");
    discovery.start().await?;

    for _ in 0..10 {
        sleep(Duration::from_millis(1000)).await;
        let peers = discovery.get_peers().await;
        if !peers.is_empty() {
            println!("\nFound {} peer(s):", peers.len());
            for p in &peers {
                println!(
                    "  -> [{:?}] {} ({}) at {}:{}",
                    p.device_type, p.alias, p.device_model, p.address, p.port
                );
            }
        }
    }

    discovery.stop().await;
    Ok(())
}

async fn cmd_listen(port: u16, download_dir: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = DropConfig::default();
    config.port = port;
    config.download_dir = download_dir.clone();
    config.auto_accept = true;
    config.alias = format!("LibreDrop Node ({})", get_local_ip());
    config.device_type = DeviceType::Server;

    let discovery = DiscoveryManager::new(config.clone());
    discovery.start().await?;

    let transfer = TransferManager::new(config);
    let mut progress_rx = transfer.subscribe_progress();

    transfer.set_message_handler(Arc::new(|sender, msg| {
        println!("\n[Received Message from {}]:\n{}", sender, msg);
    })).await;

    transfer.start_server().await?;

    println!("LibreDrop listener running on 0.0.0.0:{}", port);
    println!("Saving files to: {}", download_dir);
    println!("Auto-accept enabled. Waiting for transfers (press Ctrl+C to stop)...");

    while let Ok(progress) = progress_rx.recv().await {
        if progress.is_completed {
            println!(
                "\nTransfer completed: {} (total: {})",
                progress.current_file_name,
                format_bytes(progress.total_bytes)
            );
        } else {
            let percent = (progress.progress_ratio * 100.0) as u32;
            print!(
                "\rReceiving {}: [{:>3}%] {} / {} @ {}",
                progress.current_file_name,
                percent,
                format_bytes(progress.total_bytes_transferred),
                format_bytes(progress.total_bytes),
                format_speed(progress.speed_bytes_sec)
            );
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }

    Ok(())
}

async fn cmd_send_files(
    target_ip: &str,
    target_port: u16,
    files: Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = DropConfig::default();
    config.alias = format!("LibreDrop Sender ({})", get_local_ip());
    config.device_type = DeviceType::Cli;

    let (progress_tx, mut progress_rx) = tokio::sync::broadcast::channel(64);

    println!("Connecting to {}:{}...", target_ip, target_port);
    println!("Files to transfer (lossless zero-downscale): {:?}", files);

    let send_task = tokio::spawn({
        let target_ip = target_ip.to_string();
        let config = config.clone();
        let files = files.clone();
        let tx = progress_tx.clone();
        async move {
            TransferManager::send_transfer(&target_ip, target_port, &config, None, files, tx).await
        }
    });

    while let Ok(progress) = progress_rx.recv().await {
        if progress.is_completed {
            println!(
                "\nTransfer successful: {} (total: {})",
                progress.current_file_name,
                format_bytes(progress.total_bytes)
            );
            break;
        } else {
            let percent = (progress.progress_ratio * 100.0) as u32;
            print!(
                "\rSending {}: [{:>3}%] {} / {} @ {}",
                progress.current_file_name,
                percent,
                format_bytes(progress.total_bytes_transferred),
                format_bytes(progress.total_bytes),
                format_speed(progress.speed_bytes_sec)
            );
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }

    match send_task.await? {
        Ok(_) => {
            println!("All files transferred successfully bit-for-bit!");
            Ok(())
        }
        Err(e) => {
            eprintln!("\nTransfer failed: {}", e);
            Err(e)
        }
    }
}

async fn cmd_send_message(
    target_ip: &str,
    target_port: u16,
    message: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = DropConfig::default();
    config.alias = format!("LibreDrop Sender ({})", get_local_ip());
    config.device_type = DeviceType::Cli;

    let (progress_tx, _) = tokio::sync::broadcast::channel(16);

    println!("Dropping message to {}:{}...", target_ip, target_port);
    TransferManager::send_transfer(target_ip, target_port, &config, Some(message), vec![], progress_tx).await?;
    println!("Message dropped successfully!");
    Ok(())
}

async fn cmd_web(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = DropConfig::default();
    config.port = port;
    config.download_dir = "./Downloads".to_string();

    let server = WebDropServer::new(config, port);
    let local_ip = get_local_ip();

    server.start().await?;
    println!("LibreDrop Web Gateway active!");
    println!("Open this URL in any browser on the local Wi-Fi:");
    println!("  -> http://{}:{}", local_ip, port);
    println!("  -> http://127.0.0.1:{}", port);
    println!("Press Ctrl+C to stop...");

    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
