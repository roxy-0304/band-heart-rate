[English](README_EN.md) or [中文](README.md)

## Disclaimer

> This project is forked from [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate), code written by AI.

# Mi Band Heart Rate Demo Program

A demo program for receiving Mi Band "Exercise Heart Rate Broadcast". You need to enable the broadcast function in the band's settings - Heart Rate Broadcast.

Welcome to secondary development.

## Supported Platforms

Uses the `bluest` crate. Here is its description:

> Bluest is a cross-platform Rust low-power Bluetooth (BLE) library. Currently supports Windows (version 10 and above), MacOS/iOS, and Linux. Android support is planned.

Therefore, support:

- Windows 10/11
- MacOS/iOS
- Linux

## Supported Mi Bands

Tested on Mi Band 10.

## Features

- **Real-time Heart Rate Display**: Uses `print!` + `flush()` to refresh in real-time on the same line
- **Web Interface**: Real-time display of heart rate data and sensor contact status in the browser
- **Custom Styles**: Supports injecting custom CSS in the Web interface
- **Cross-platform Support**: Windows, macOS/iOS, Linux

### Accessing the Web Interface

Open in your browser: `http://127.0.0.1:3030`

The port number will be displayed when the program starts. The default port is 3030, and it will automatically switch to another port if a conflict occurs.

View heart rate data and sensor status in real-time.

## Tips
The default window size is 1920x1080. If it is not fully displayed, please adjust the window size.

## Screenshot

![Alt text](doc/1.png)
![Alt text](doc/2.gif)
