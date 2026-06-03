[English](README_EN.md) or [中文](README.md)

## ⚠️ Disclaimer

> This project is forked from [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate), code written by AI.

---

## 📋 Table of Contents

- [About](#about)
- [✨ Features](#features)
- [🖥️ Native Interface](#native-interface)
- [📺 Live Stream Overlay](#live-stream-overlay)
- [📦 Quick Start](#quick-start)
- [🚀 Usage Guide](#usage-guide)
- [⚙️ Environment Variables](#environment-variables)
- [📁 Project Structure](#project-structure)
- [🖼️ Screenshots](#screenshots)
- [Supported Platforms](#supported-platforms)
- [Compatible Devices](#compatible-devices)

---

## About

**Band Heart Rate Monitor** is a native desktop heart rate monitoring application built with Rust and the Slint GUI framework. It receives real-time heart rate data from wearable devices via the standard BLE Heart Rate Service (UUID 0x180D). The Slint-based native rendering engine provides low memory usage and high frame rate. A built-in HTTP server allows remote viewing or integration with live stream overlays.

You need to enable the heart rate broadcast function in your wearable device's settings.

> 💡 Latest builds can be downloaded from the [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) page.

---

## ✨ Features

- **Slint Native Window** — Pixel-level rendering, minimal CPU and memory usage, high frame rate
- **Real-time Heart Rate Display** — Large digits with real-time refresh
- **Heart Rate Zone Detection** — Automatically identifies Warmup / Fat Burn / Aerobic / Limit zones with color-coded indicators
- **Live Statistics** — Automatically tracks min / max / average heart rate, with one-click reset
- **Connection Status** — Shows Connected / Scanning / Disconnected and Bluetooth error states
- **HTTP API Server** — Supports REST and SSE real-time push, easily integrated with browsers or multiple clients
- **Auto Reconnect** — Automatically scans and reconnects when the device disconnects, with exponential backoff
- **Cross-platform** — Windows, MacOS, Linux

---

## 🖥️ Native Interface

The main interface is a Slint native rendering window offering efficient real-time monitoring:

- Large real-time heart rate display
- Heart rate zone color badges (Warmup / Fat Burn / Aerobic / Limit)
- Min / Max / Average statistics panel
- Reset statistics button
- No browser or WebView required, reducing memory and CPU usage

---

## 📺 Live Stream Overlay

The built-in HTTP server provides the following endpoints for integration:

| Endpoint | Description |
|----------|-------------|
| GET /heart-rate | Get current heart rate as JSON |
| GET /heart-rate-stream | SSE real-time push of heart rate data |
| GET /health | Health check and version info |

Default listen address:

```
http://127.0.0.1:3030
```

(Automatically uses a random port if 3030 is occupied. Check startup logs for the actual address.)

Access this URL in a browser or OBS browser source to display heart rate data. Supports custom frontend styles.

---

## 📦 Quick Start

### Download Pre-built Binary

1. Go to the [GitHub Releases page](https://github.com/Roxy-0304/band-heart-rate/releases)
2. Download the latest executable for your platform (e.g., `band-heart-rate.exe` for Windows)
3. Run it directly

### Build from Source

**Requirements:**

- [Rust toolchain](https://www.rust-lang.org/tools/install) (recommended via rustup)
- Bluetooth enabled on your device (Windows requires Bluetooth support)

**Build steps:**

```bash
# Clone the repository
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# Build default version with native GUI
cargo build --release

# Or build headless version (HTTP API only, no GUI)
cargo build --release --no-default-features --features=""
```

---

## 🚀 Usage Guide

1. Enable **Heart Rate Broadcast** in your **band/watch settings**
2. Make sure Bluetooth is enabled on your device
3. Run the compiled executable
4. The program will automatically scan for nearby heart rate broadcasting devices and connect
5. Heart rate data will be displayed in real-time in the native window, and also accessible via HTTP API
6. Access `http://127.0.0.1:3030/heart-rate-stream` in a browser to view the real-time data stream

---

## ⚙️ Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MIBAND_ALLOWED_DEVICES` | Comma-separated device name keywords for allowed connections (case-insensitive) | `band,amazfit,watch,mi` |

**Example:**

```bash
# Only allow devices containing "mi" or "honor"
set MIBAND_ALLOWED_DEVICES=mi,honor
band-heart-rate.exe
```

---

## 📁 Project Structure

```
band-heart-rate/
├── src/
│   ├── main.rs              # Entry point and initialization
│   ├── gui.rs               # Slint native GUI window logic
│   ├── ble.rs               # BLE heart rate device communication
│   ├── server.rs            # HTTP API server (REST and SSE)
│   ├── types.rs             # Shared data structure definitions
│   └── macros.rs            # Utility macro definitions
├── ui/
│   └── app.slint            # Slint UI layout definition
├── doc/
│   ├── 1.png                # Desktop main interface screenshot
│   ├── 2.gif                # Live demo animation
│   └── 3.png                # Desktop feature demo screenshot
├── icons/
│   └── icon.ico             # App icon
├── build.rs                 # Build script
├── Cargo.toml               # Rust project configuration and dependencies
├── Cargo.lock               # Dependency lock file
├── README.md                # Chinese documentation
├── README_EN.md             # English documentation
└── LICENSE                  # Open source license
```

---

## 🖼️ Screenshots

![Desktop Main Interface](doc/1.png)

![Desktop Feature Demo](doc/3.png)

![Live Demo](doc/2.gif)

---

## Supported Platforms

Based on the Slint framework and `bluest` BLE library:

- Windows 10/11
- MacOS
- Linux

---

## Compatible Devices

Compatible with any wearable device that supports the standard BLE Heart Rate Service (UUID 0x180D), including but not limited to:

- **Xiaomi Mi Band** series (tested on Mi Band 10)
- **Honor Band** series
- **Huawei Band/Watch** series
- **Amazfit** devices
- **Apple Watch**
- Other sports watches / chest straps that support BLE heart rate broadcasting

> Enable the "Heart Rate Broadcast" feature in your device settings to be detected by this program.
