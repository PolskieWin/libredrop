# LibreDrop

LibreDrop is an open-source, local network file and message transfer application built in Rust and Kotlin. It enables direct peer-to-peer data transfers between devices on the same Wi-Fi network or mobile hotspot with zero compression, zero downscaling, and no external relays or subscriptions.

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
