# IRIS — LAN Screen & File Sharing

Cross-platform screen sharing and file transfer over LAN, built with **Rust** (core engine) + **Flutter** (UI) via FFI.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                    Flutter UI                         │
│     Discovery │ Screen View │ File Transfer           │
├──────────────────────────────────────────────────────┤
│                 dart:ffi Bindings                     │
├──────────────────────────────────────────────────────┤
│                  iris_core (Rust)                     │
│  ┌──────────┬────────────┬────────────┬───────────┐  │
│  │Discovery │  UDP Tx    │  TCP Tx    │  Screen   │  │
│  │ (mDNS)   │  (Stream)  │ (Ctrl+File)│  Capture  │  │
│  └──────────┴────────────┴────────────┴───────────┘  │
└──────────────────────────────────────────────────────┘
```

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **UI** | Flutter (Dart) | Cross-platform UI: Windows, macOS, Android |
| **FFI** | dart:ffi | Type-safe Rust ↔ Dart bindings |
| **Transport** | UDP + TCP | UDP for low-latency screen streaming; TCP for reliable control & file transfer |
| **Discovery** | mDNS (mdns-sd) | Zero-config LAN peer discovery |
| **Capture** | DXGI / CGDisplay | Platform-native screen capture |

## Project Structure

```
IRIS_WS/
├── iris_core/                  # Rust core library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Crate root, re-exports
│       ├── ffi.rs              # C-compatible FFI for Flutter
│       ├── protocol.rs         # UDP/TCP message definitions
│       ├── discovery.rs        # mDNS LAN peer discovery
│       ├── transport/
│       │   ├── udp.rs          # UDP frame streaming
│       │   └── tcp.rs          # TCP control + file messages
│       ├── screen/
│       │   ├── capture.rs      # Platform screen capture
│       │   └── encoder.rs      # JPEG/LZ4 frame encoding
│       └── file_transfer.rs    # File chunk transfer logic
│
├── iris_flutter/               # Flutter application
│   ├── pubspec.yaml
│   ├── lib/
│   │   ├── main.dart
│   │   ├── ffi/
│   │   │   └── iris_bindings.dart   # dart:ffi type definitions
│   │   ├── screens/
│   │   │   ├── discovery_screen.dart
│   │   │   ├── screen_share_screen.dart
│   │   │   └── file_transfer_screen.dart
│   │   ├── services/
│   │   │   └── iris_service.dart     # High-level Dart service
│   │   └── widgets/
│   │       └── device_tile.dart
│   └── native/
│       └── Cargo.toml
│
├── build.sh                    # Linux/macOS build script
├── build.bat                   # Windows build script
├── .gitignore
└── README.md
```

## Protocol

### UDP (Port 21000) — Screen Streaming

| Message | Direction | Description |
|---------|-----------|-------------|
| `FrameChunk` | S→R | Encoded frame chunk with ID, dimensions, encoding |
| `KeyFrameRequest` | R→S | Request a full keyframe |
| `Ack` | R→S | Acknowledge received chunks |
| `StreamControl` | R→S | Pause/Resume/Stop stream |

Frame encodings: `RawRgba` (0), `Jpeg` (1), `Lz4` (2)

### TCP (Port 21001) — Control & File Transfer

| Message | Purpose |
|---------|---------|
| `Hello` / `HelloAck` | Handshake with capabilities |
| `DeviceAnnounce` / `DeviceList` | Manual discovery fallback |
| `StartScreenShare` / `ScreenShareStarted` | Negotiate screen stream |
| `FileListRequest` / `FileListResponse` | Browse remote files |
| `FileChunk` / `FileTransferComplete` | Reliable file chunks |
| `Ping` / `Pong` | Keep-alive |

## Build & Run

### Prerequisites

- **Rust** (1.75+) with `rustup`
- **Flutter** (3.16+) with desktop support enabled
- **Android NDK** (for Android builds)

### Quick Start

```bash
# 1. Build Rust core
./build.sh        # Linux/macOS
build.bat         # Windows

# 2. Run Flutter app
cd iris_flutter
flutter pub get
flutter run -d windows    # or macos, android
```

### Platform-specific notes

| Platform | Screen Capture | Status |
|----------|---------------|--------|
| Windows | DXGI Desktop Duplication | Scaffold |
| macOS | CGDisplay | Implemented |
| Android | MediaProjection (Flutter side) | Stub |
| Linux | PipeWire / X11 SHM | Stub |

## Key Design Decisions

1. **UDP for streaming** — Screen frames are time-sensitive and loss-tolerant. UDP avoids TCP head-of-line blocking.
2. **TCP for control/file** — Commands and file data need reliable, ordered delivery.
3. **mDNS discovery** — No central server needed; peers find each other via multicast DNS.
4. **JSON over TCP** — Human-debuggable control messages; Bincode for compact UDP frames.
5. **FFI via C ABI** — Simple `extern "C"` functions with JSON string passing for maximum compatibility.

## License

MIT
