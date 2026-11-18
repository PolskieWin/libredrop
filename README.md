# LibreDrop

LibreDrop is an open-source, local network file and message transfer application built in Rust and Kotlin. It enables direct peer-to-peer data transfers between devices on the same Wi-Fi network or mobile hotspot with zero compression, zero downscaling, and no external relays or subscriptions.

---

## Direct Download (Latest Release v1.0.0)

| Package | Architecture | Direct Download Link | Size |
|---|---|---|---|
| **Universal APK** | All Android Devices (arm64, armv7, x86_64) | [Download Universal APK](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/LibreDrop-v1.0.0-universal.apk) | ~3.5 MB |
| **ARMv8 64-bit APK** | Modern 64-bit Smartphones | [Download arm64-v8a APK](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/LibreDrop-v1.0.0-arm64-v8a.apk) | ~1.9 MB |
| **ARMv7 32-bit APK** | Legacy 32-bit Phones & Tablets | [Download armeabi-v7a APK](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/LibreDrop-v1.0.0-armeabi-v7a.apk) | ~1.7 MB |
| **x86_64 APK** | Android Emulators & Chromebooks | [Download x86_64 APK](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/LibreDrop-v1.0.0-x86_64.apk) | ~2.0 MB |
| **Linux aarch64 CLI** | Termux & ARM64 Linux Servers | [Download aarch64 Tarball](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/libredrop-v1.0.0-linux-aarch64.tar.gz) | ~843 KB |
| **Linux x86_64 CLI** | 64-bit Linux Desktops & Servers | [Download x86_64 Binary](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/libredrop-cli-linux-x86_64) | ~941 KB |
| **Windows x86_64 Bundle** | Windows 10 & 11 (CLI + Web Gateway + Launchers) | [Download Windows Bundle](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/LibreDrop-v1.0.0-windows-x86_64.zip) | ~1.1 MB |

All packages can also be found on the [GitHub Releases Page](https://github.com/PolskieWin/libredrop/releases/tag/v1.0.0).

---

## Windows Quick Start

1. Download [LibreDrop-v1.0.0-windows-x86_64.zip](https://github.com/PolskieWin/libredrop/releases/download/v1.0.0/LibreDrop-v1.0.0-windows-x86_64.zip) and extract it anywhere on your PC.
2. **Web Gateway**: Double-click `run-libredrop-web.bat` to launch the background server and open the Web UI directly in your default browser.
3. **Receiver Daemon**: Double-click `run-libredrop-listen.bat` to receive files from phones and laptops directly into your Windows `Downloads` folder.
4. **Command Prompt / PowerShell**: Run `powershell -ExecutionPolicy Bypass -File install-windows.ps1` to add `libredrop` to your PATH and create a Desktop shortcut.

---

## Key Technical Specifications

- **Native Rust Core Engine**: Uses non-blocking asynchronous TCP/UDP networking (Tokio) for high-throughput streaming with 512 KB chunk buffering and `TCP_NODELAY`.
- **Zero Downscaling**: Files are transmitted in raw binary mode. Media (RAW photos, 4K/8K 60fps video, lossless audio, APKs) is preserved byte-for-byte.
- **Integrity Verification**: Real-time SHA-256 calculation verifies stream integrity on receipt.
- **Local Discovery Protocol**: UDP beaconing on port 53318 for automatic device announcement without cloud servers.
- **P2P Transfer Port**: TCP 53317 for binary streaming transfers.
- **Web Gateway Mode**: Built-in HTTP server on port 53319 allowing any standard web browser (iOS, macOS, Windows, Linux) to upload and download files without installing an application.
- **Android Client (Kotlin + Jetpack Compose)**:
  - Material 3 interface with dynamic theming.
  - Interactive radar scanner visualizer.
  - Background foreground service to prevent transfer interruptions on large files.
  - System share intent receiver (`ACTION_SEND` and `ACTION_SEND_MULTIPLE`).
  - Fallback engine for execution environments without native binaries.
- **CLI Utility**: Standalone `libredrop` executable for command-line operation and headless servers.

## Workspace Layout

- `libredrop-core`: Shared Rust library handling discovery, transfer protocols, SHA-256 hashing, and Web Drop server.
- `libredrop-engine`: Rust `cdylib` exposing JNI interfaces for the Android runtime.
- `libredrop-cli`: Command-line tool for scanning, listening, and transferring via terminal.
- `app`: Modern Android application built with Kotlin, Coroutines, StateFlow, and Jetpack Compose.

## CLI Usage

```bash
# Display local IP and configuration
libredrop status

# Scan local subnet for active LibreDrop peers
libredrop scan

# Start listening daemon to receive files into a directory
libredrop listen --port 53317 --dir ./Downloads

# Send files to a peer IP
libredrop send 192.168.1.50 video_4k.mp4 photo.raw

# Send a text message to a peer IP
libredrop msg 192.168.1.50 "Meeting room link: https://..."

# Start local web gateway for browser-based transfers
libredrop web 53319
```

## Protocol Architecture

1. **Beacon Announcement**: Nodes broadcast JSON metadata packets over UDP (`255.255.255.255:53318`).
2. **Transfer Handshake**: Sender opens TCP stream to recipient on port 53317 and sends a length-prefixed frame (`0x01`) containing transfer metadata (file names, sizes, or text payload).
3. **Response Frame**: Recipient responds with frame `0x02` indicating acceptance or rejection.
4. **Pipelined Data Streaming**: On acceptance, files are streamed in consecutive `0x11` frames with trailing SHA-256 checksums (`0x12`) and recipient acknowledgments (`0x13`).

## License

MIT License. Free and open source.
