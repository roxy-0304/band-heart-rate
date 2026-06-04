[English](README_EN.md) | [中文](README.md)

[![CI](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml/badge.svg)](https://github.com/Roxy-0304/band-heart-rate/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Roxy-0304/band-heart-rate)](https://github.com/Roxy-0304/band-heart-rate/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ 免责声明

> 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。

## 简介

**Band Heart Rate Monitor** 是一款基于 Rust 和 Slint 的原生桌面心率监测应用，通过标准 BLE 心率服务（UUID 0x180D）接收穿戴设备的实时心率数据。内置 HTTP 服务器，支持 REST 和 SSE 实时推送，便于集成到直播叠加层。

需要在手环/手表的设置中开启心率广播功能。

> 💡 最新版本可从 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载。

## 功能特性

- **Slint 原生窗口** — CPU 和内存占用极低，高帧率刷新
- **实时心率显示** — 大数字实时刷新，自动识别热身/燃脂/有氧/极限四区
- **实时统计** — 最低/最高/平均心率，支持一键重置
- **HTTP API** — REST 接口 + SSE 实时推送
- **自动重连** — 断开后自动扫描重连，指数退避
- **跨平台** — Windows / macOS / Linux

## 快速开始

### 下载

前往 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 下载对应平台的可执行文件，直接运行即可。

### 源码编译

```bash
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# 带 GUI 版本
cargo build --release

# 纯后端版本（仅 HTTP API）
cargo build --release --no-default-features
```

**环境要求：** [Rust 工具链](https://www.rust-lang.org/tools/install)（推荐 rustup）

## 使用指南

1. 在手环/手表设置中开启 **心率广播**
2. 确保设备蓝牙已开启
3. 运行程序，自动扫描并连接心率设备
4. 心率数据在原生窗口实时显示，同时通过 HTTP API 提供访问

## HTTP API

| 接口 | 说明 |
|------|------|
| `GET /heart-rate` | 当前心率 JSON |
| `GET /heart-rate-stream` | SSE 实时心率数据流 |
| `GET /health` | 健康检查 |

默认地址 `http://127.0.0.1:3030`，端口冲突时自动使用随机端口。

## 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `MIBAND_ALLOWED_DEVICES` | 允许连接的设备名称关键词（逗号分隔） | `band,amazfit,watch,mi` |

## 兼容设备

兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备，已在 Windows 10/11、macOS、Linux 上测试。

支持的设备包括：小米手环、荣耀手环、华为手环/手表、Amazfit、Apple Watch 等。在设备设置中开启"心率广播"即可被识别。

## 截图

**后端原生界面**

![后端原生界面](doc/1.png)

**前端 Web 界面**

![前端 Web 界面](doc/2.png)

## License

[MIT](LICENSE)