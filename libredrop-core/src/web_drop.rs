use crate::models::DropConfig;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::fs::{create_dir_all, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

const WEB_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>LibreDrop Web Gateway</title>
  <style>
    :root {
      --bg: #0b0f19;
      --card-bg: #131b2e;
      --accent: #00e5ff;
      --accent-hover: #00b4d8;
      --text: #f0f4f8;
      --text-muted: #94a3b8;
      --border: #1e293b;
      --success: #10b981;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      display: flex;
      flex-direction: column;
      align-items: center;
      min-height: 100vh;
      padding: 24px 16px;
    }
    .header {
      text-align: center;
      margin-bottom: 28px;
    }
    .logo {
      font-size: 32px;
      font-weight: 800;
      letter-spacing: -0.5px;
      color: var(--accent);
      margin-bottom: 6px;
    }
    .badge {
      display: inline-block;
      font-size: 11px;
      font-weight: 700;
      text-transform: uppercase;
      padding: 3px 8px;
      border-radius: 12px;
      background: rgba(0, 229, 255, 0.15);
      color: var(--accent);
      margin-bottom: 8px;
    }
    .subtitle {
      font-size: 14px;
      color: var(--text-muted);
    }
    .container {
      width: 100%;
      max-width: 580px;
      display: flex;
      flex-direction: column;
      gap: 20px;
    }
    .card {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 24px;
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
    }
    .card-title {
      font-size: 18px;
      font-weight: 700;
      margin-bottom: 14px;
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .drop-zone {
      border: 2px dashed var(--accent);
      border-radius: 12px;
      padding: 36px 20px;
      text-align: center;
      cursor: pointer;
      background: rgba(0, 229, 255, 0.03);
      transition: all 0.2s ease;
    }
    .drop-zone:hover, .drop-zone.dragover {
      background: rgba(0, 229, 255, 0.08);
      border-color: #38bdf8;
    }
    .drop-zone-text {
      font-size: 15px;
      color: var(--text);
      margin-bottom: 6px;
      font-weight: 600;
    }
    .drop-zone-sub {
      font-size: 12px;
      color: var(--text-muted);
    }
    input[type="file"] {
      display: none;
    }
    textarea {
      width: 100%;
      height: 110px;
      background: #090d16;
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 12px;
      color: var(--text);
      font-size: 14px;
      resize: vertical;
      margin-bottom: 12px;
      outline: none;
    }
    textarea:focus {
      border-color: var(--accent);
    }
    .btn {
      display: inline-block;
      width: 100%;
      background: var(--accent);
      color: #050b14;
      font-weight: 700;
      font-size: 14px;
      padding: 12px 20px;
      border-radius: 10px;
      border: none;
      cursor: pointer;
      text-align: center;
      transition: background 0.2s;
    }
    .btn:hover {
      background: var(--accent-hover);
    }
    .status {
      margin-top: 12px;
      font-size: 13px;
      text-align: center;
      min-height: 20px;
    }
    .status.success { color: var(--success); }
    .status.error { color: #ef4444; }
    .progress-bar-container {
      width: 100%;
      height: 8px;
      background: #090d16;
      border-radius: 4px;
      overflow: hidden;
      margin-top: 14px;
      display: none;
    }
    .progress-bar {
      width: 0%;
      height: 100%;
      background: var(--accent);
      transition: width 0.1s linear;
    }
  </style>
</head>
<body>
  <div class="header">
    <div class="badge">100% Offline LAN • Lossless</div>
    <div class="logo">LibreDrop</div>
    <p class="subtitle">Bit-for-bit file & message transfer with zero downscaling and zero paywalls</p>
  </div>

  <div class="container">
    <div class="card">
      <div class="card-title">Drop Files to Device</div>
      <div class="drop-zone" id="dropZone">
        <div class="drop-zone-text">Click or Drag & Drop Any Files Here</div>
        <div class="drop-zone-sub">Original full resolution (RAW, 4K/8K, APK, ZIP) — No compression</div>
      </div>
      <input type="file" id="fileInput" multiple>
      <div class="progress-bar-container" id="progressContainer">
        <div class="progress-bar" id="progressBar"></div>
      </div>
      <div class="status" id="fileStatus"></div>
    </div>

    <div class="card">
      <div class="card-title">Drop Quick Note / Message</div>
      <textarea id="messageInput" placeholder="Type or paste text, URLs, clipboard notes here..."></textarea>
      <button class="btn" id="sendMsgBtn">Send Message</button>
      <div class="status" id="msgStatus"></div>
    </div>
  </div>

  <script>
    const dropZone = document.getElementById('dropZone');
    const fileInput = document.getElementById('fileInput');
    const progressContainer = document.getElementById('progressContainer');
    const progressBar = document.getElementById('progressBar');
    const fileStatus = document.getElementById('fileStatus');

    dropZone.addEventListener('click', () => fileInput.click());
    dropZone.addEventListener('dragover', (e) => { e.preventDefault(); dropZone.classList.add('dragover'); });
    dropZone.addEventListener('dragleave', () => dropZone.classList.remove('dragover'));
    dropZone.addEventListener('drop', (e) => {
      e.preventDefault();
      dropZone.classList.remove('dragover');
      if (e.dataTransfer.files.length > 0) {
        uploadFiles(e.dataTransfer.files);
      }
    });

    fileInput.addEventListener('change', () => {
      if (fileInput.files.length > 0) {
        uploadFiles(fileInput.files);
      }
    });

    function uploadFiles(files) {
      fileStatus.className = 'status';
      fileStatus.textContent = `Uploading ${files.length} file(s)...`;
      progressContainer.style.display = 'block';
      progressBar.style.width = '0%';

      let completed = 0;
      for (let i = 0; i < files.length; i++) {
        const file = files[i];
        const xhr = new XMLHttpRequest();
        xhr.open('POST', '/api/upload?name=' + encodeURIComponent(file.name), true);

        xhr.upload.onprogress = (e) => {
          if (e.lengthComputable) {
            const percent = Math.round((e.loaded / e.total) * 100);
            progressBar.style.width = percent + '%';
          }
        };

        xhr.onload = () => {
          if (xhr.status === 200) {
            completed++;
            if (completed === files.length) {
              fileStatus.className = 'status success';
              fileStatus.textContent = 'Transfer complete! Lossless bit-for-bit copy saved.';
              setTimeout(() => { progressContainer.style.display = 'none'; }, 2000);
            }
          } else {
            fileStatus.className = 'status error';
            fileStatus.textContent = 'Upload failed: ' + xhr.responseText;
          }
        };

        xhr.onerror = () => {
          fileStatus.className = 'status error';
          fileStatus.textContent = 'Network error during file drop.';
        };

        xhr.send(file);
      }
    }

    const sendMsgBtn = document.getElementById('sendMsgBtn');
    const messageInput = document.getElementById('messageInput');
    const msgStatus = document.getElementById('msgStatus');

    sendMsgBtn.addEventListener('click', () => {
      const text = messageInput.value.trim();
      if (!text) return;
      msgStatus.className = 'status';
      msgStatus.textContent = 'Sending note...';

      fetch('/api/message', {
        method: 'POST',
        headers: { 'Content-Type': 'text/plain; charset=utf-8' },
        body: text
      })
      .then(res => {
        if (res.ok) {
          msgStatus.className = 'status success';
          msgStatus.textContent = 'Message dropped successfully!';
          messageInput.value = '';
          setTimeout(() => { msgStatus.textContent = ''; }, 3000);
        } else {
          msgStatus.className = 'status error';
          msgStatus.textContent = 'Failed to drop message.';
        }
      })
      .catch(() => {
        msgStatus.className = 'status error';
        msgStatus.textContent = 'Network error sending message.';
      });
    });
  </script>
</body>
</html>
"#;

fn decode_url(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                result.push(val);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            result.push(b' ');
            i += 1;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub struct WebDropServer {
    config: Arc<RwLock<DropConfig>>,
    is_running: Arc<AtomicBool>,
    port: u16,
}

impl WebDropServer {
    pub fn new(config: DropConfig, web_port: u16) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            is_running: Arc::new(AtomicBool::new(false)),
            port: web_port,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.is_running.store(true, Ordering::SeqCst);

        let bind_addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&bind_addr).await?;

        let is_running = self.is_running.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                tokio::select! {
                    res = listener.accept() => {
                        if let Ok((mut stream, _)) = res {
                            let conf = config.clone();
                            tokio::spawn(async move {
                                let _ = Self::handle_http(&mut stream, conf).await;
                            });
                        }
                    }
                    _ = sleep(Duration::from_millis(500)) => {}
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    async fn handle_http(
        stream: &mut TcpStream,
        config: Arc<RwLock<DropConfig>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut header_buf = [0u8; 4096];
        let n = stream.read(&mut header_buf).await?;
        if n == 0 {
            return Ok(());
        }

        let header_str = String::from_utf8_lossy(&header_buf[..n]);
        let mut lines = header_str.lines();
        let req_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = req_line.split_whitespace().collect();

        if parts.len() < 2 {
            return Ok(());
        }

        let method = parts[0];
        let path = parts[1];

        if method == "GET" && (path == "/" || path == "/index.html") {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                WEB_HTML.len(),
                WEB_HTML
            );
            stream.write_all(response.as_bytes()).await?;
            stream.flush().await?;
        } else if method == "POST" && path.starts_with("/api/upload") {
            let file_name = if let Some(query_idx) = path.find("?name=") {
                let encoded = &path[query_idx + 6..];
                decode_url(encoded)
            } else {
                "dropped_upload.bin".to_string()
            };

            let sanitized_name = Path::new(&file_name)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "dropped_upload.bin".to_string());

            let download_dir = config.read().await.download_dir.clone();
            create_dir_all(&download_dir).await?;
            let dest_path = PathBuf::from(&download_dir).join(&sanitized_name);

            // Find start of HTTP body
            let delimiter = b"\r\n\r\n";
            let mut body_start_idx = 0;
            for i in 0..n.saturating_sub(3) {
                if &header_buf[i..i + 4] == delimiter {
                    body_start_idx = i + 4;
                    break;
                }
            }

            // Extract Content-Length
            let mut content_length = 0usize;
            for line in header_str.lines() {
                if line.to_lowercase().starts_with("content-length:") {
                    if let Some(val) = line.split(':').nth(1) {
                        content_length = val.trim().parse().unwrap_or(0);
                    }
                }
            }

            let mut out_file = File::create(&dest_path).await?;
            let mut written = 0usize;

            if body_start_idx > 0 && body_start_idx < n {
                let initial_chunk = &header_buf[body_start_idx..n];
                out_file.write_all(initial_chunk).await?;
                written += initial_chunk.len();
            }

            let mut chunk = [0u8; 64 * 1024];
            while written < content_length {
                let bytes_to_read = std::cmp::min(chunk.len(), content_length - written);
                let read_n = stream.read(&mut chunk[..bytes_to_read]).await?;
                if read_n == 0 {
                    break;
                }
                out_file.write_all(&chunk[..read_n]).await?;
                written += read_n;
            }
            out_file.flush().await?;

            let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nOK";
            stream.write_all(resp.as_bytes()).await?;
            stream.flush().await?;
        } else if method == "POST" && path == "/api/message" {
            let delimiter = b"\r\n\r\n";
            let mut body_start_idx = 0;
            for i in 0..n.saturating_sub(3) {
                if &header_buf[i..i + 4] == delimiter {
                    body_start_idx = i + 4;
                    break;
                }
            }

            let msg = if body_start_idx > 0 && body_start_idx < n {
                String::from_utf8_lossy(&header_buf[body_start_idx..n]).to_string()
            } else {
                String::new()
            };

            // Save message to downloads as text note
            let download_dir = config.read().await.download_dir.clone();
            create_dir_all(&download_dir).await?;
            let note_path = PathBuf::from(&download_dir).join("dropped_notes.txt");
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(note_path)
                .await?;
            file.write_all(format!("[Web Drop]: {}\n", msg).as_bytes()).await?;
            file.flush().await?;

            let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nOK";
            stream.write_all(resp.as_bytes()).await?;
            stream.flush().await?;
        } else {
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(resp.as_bytes()).await?;
            stream.flush().await?;
        }

        Ok(())
    }
}
