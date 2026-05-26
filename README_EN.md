[English](README_EN.md) or [中文](README.md)

## ⚠️ Disclaimer

> This project is forked from [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate), code written by AI.
---

## 📋 Table of Contents

- [About](#about)
- [✨ Features](#features)
- [🖥️ Web UI](#web-ui)
  - [Custom Styles](#custom-styles)
- [📺 OBS Live Stream Overlay Setup](#obs-live-stream-overlay-setup)
- [📦 Quick Start](#quick-start)
  - [Download Pre-built Binary](#download-pre-built-binary)
  - [Build from Source](#build-from-source)
- [🚀 Usage Guide](#usage-guide)
- [⚙️ Environment Variables](#environment-variables)
- [📁 Project Structure](#project-structure)
- [🖼️ Screenshots](#screenshots)
- [Supported Platforms](#supported-platforms)
- [Compatible Devices](#compatible-devices)

---

## About

A BLE heart rate monitor demo program that receives heart rate broadcast data via the standard BLE Heart Rate Service (UUID 0x180D). You need to enable the heart rate broadcast function in your wearable device's settings.

> 💡 Latest builds can be downloaded from the [GitHub Releases](https://github.com/Roxy-0304/miband-heart-rate/releases) page.

---

## ✨ Features

- **Real-time Heart Rate Display**: Uses `print!` + `flush()` to refresh in real-time on the same line in terminal
- **Web Interface**: Real-time display of heart rate data and sensor contact status in the browser
- **Custom Styles**: Supports injecting custom CSS in the Web interface
- **OBS Live Stream Compatible**: Page designed specifically for live stream overlay scenarios
- **Auto Reconnect**: Automatically scans and reconnects when device disconnects
- **Cross-platform Support**: Windows, macOS/iOS, Linux

---

## 🖥️ Web UI

The program automatically starts a web server after launching. The default address is:

```
http://127.0.0.1:3030
```

(If the port is occupied, it will automatically switch to another port. The startup message will indicate the actual port.)

Web page features:
- Designed for **1920x1080** resolution, suitable for fullscreen display
- **Transparent background** — can be overlaid directly onto live streams without a green screen
- Heart icon with **pulsing animation**
- Heart rate number displayed in **Orbitron font** for a tech-forward look
- Number appears **dimmed** when not connected

### Custom Styles

You can customize the appearance by injecting custom CSS, either via browser developer tools or the OBS browser source CSS option.

Example color override:
```css
:root {
  --red: #00FF00;   /* Change to green */
}
```

---

## 📺 OBS Live Stream Overlay Setup

The Web UI is designed for live stream overlays and can be easily integrated into OBS Studio.

**Setup steps:**

1. Run the program and ensure the Web UI is accessible
2. In OBS, click `+` → **Browser** to add a browser source
3. Configure the source properties:
   - **URL**: `http://127.0.0.1:3030`
   - **Width**: `1920`
   - **Height**: `1080`
4. Click OK to finish


The page has a transparent background by default — no chroma key or green screen setup needed.

---

## 📦 Quick Start

### Download Pre-built Binary

1. Go to the [GitHub Releases page](https://github.com/Roxy-0304/miband-heart-rate/releases)
2. Download the latest `miband-heart-rate.exe`
3. Run it directly

### Build from Source

**Requirements:**

- [Rust toolchain](https://www.rust-lang.org/tools/install) (recommended via rustup)

**Build steps:**

```bash
# Clone the repository
git clone https://github.com/Roxy-0304/miband-heart-rate.git
cd miband-heart-rate

# Build release version
cargo build --release

# The executable is at target/release/miband-heart-rate.exe
```

---

## 🚀 Usage Guide

1. Enable **Heart Rate Broadcast** in your **band/watch settings**
2. Make sure Bluetooth is enabled on your device
3. Run the program:
   ```bash
   # If building from source
   cargo run --release

   # Or run the downloaded exe directly
   ./miband-heart-rate.exe
   ```
4. The program will automatically scan for nearby heart rate broadcasting devices and connect
5. Heart rate data will be displayed in real-time in the terminal, and you can access the Web UI in your browser

**Tip:** The default window size is 1920x1080. If the display is incomplete, please resize the window.

---

## ⚙️ Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MIBAND_ALLOWED_DEVICES` | Comma-separated device name keywords for allowed connections (case-insensitive) | `band,amazfit,watch` |

**Example:**

```bash
# Only allow devices containing "mi" or "honor"
set MIBAND_ALLOWED_DEVICES=mi,honor
miband-heart-rate.exe
```

---

## 📁 Project Structure

```
band-heart-rate/
├── src/
│   └── main.rs          # Main entry point (Bluetooth, Web server, data processing)
├── doc/
│   ├── 1.png            # Screenshot
│   └── 2.gif            # Animated screenshot
├── .github/
│   └── workflows/
│       └── release.yml  # GitHub Actions automated build & release
├── Cargo.toml           # Project configuration and dependencies
├── Cargo.lock           # Dependency lock file
├── README.md            # Chinese documentation
├── README_EN.md         # English documentation
└── LICENSE              # Open source license
```

---

## 🖼️ Screenshots

![Alt text](doc/1.png)
![Alt text](doc/2.gif)

---

## Supported Platforms

Uses the `bluest` crate. Here is its description:

> Bluest is a cross-platform Rust low-power Bluetooth (BLE) library. Currently supports Windows (version 10 and above), MacOS/iOS, and Linux. Android support is planned.

Therefore, support:

- Windows 10/11
- MacOS/iOS
- Linux

---

## Compatible Devices

This program is compatible with any wearable device that supports the standard BLE Heart Rate Service (UUID 0x180D), including but not limited to:

- **Xiaomi Mi Band** series (tested on Mi Band 10)
- **Honor Band** series
- **Huawei Band/Watch** series
- **Amazfit** devices
- **Apple Watch**
- Other sports watches/chest straps that support BLE heart rate broadcasting

> Enable the "Heart Rate Broadcast" feature in your device settings to be detected by this program.
