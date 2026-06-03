[English](README_EN.md) 或 [中文](README.md)

## ⚠️ 免责声明

> 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。

---

## 📋 目录

- [项目简介](#项目简介)
- [✨ 功能特性](#功能特性)
- [🖥️ 原生界面说明](#原生界面说明)
- [📺 直播叠加层说明](#直播叠加层说明)
- [📦 快速开始](#快速开始)
- [🚀 使用指南](#使用指南)
- [⚙️ 环境变量](#环境变量)
- [📁 项目结构](#项目结构)
- [🖼️ 截图](#截图)
- [支持的平台](#支持的平台)
- [兼容设备](#兼容设备)

---

## 项目简介

**Band Heart Rate Monitor** 是一款基于 Rust 和 Slint 框架的原生桌面心率监测应用，通过标准 BLE 心率服务（Heart Rate Service, UUID 0x180D）接收穿戴设备的实时心率数据。支持原生窗口显示，内存占用低，渲染高效。另有内置 HTTP 服务器，便于通过浏览器远程查看心率数据或集成到直播叠加层。

需要在手环/手表的设置中开启心率广播功能。

> 💡 最新构建版本可从 [GitHub Releases](https://github.com/Roxy-0304/band-heart-rate/releases) 页面下载。

---

## ✨ 功能特性

- **Slint 原生窗口界面** — 原生像素渲染，CPU和内存占用极低，支持高帧率刷新
- **实时心率显示** — 大数字实时刷新
- **心率区间识别** — 自动判断热身/燃脂/有氧/极限四区，彩色标识
- **实时统计** — 自动记录最低/最高/平均心率，支持一键重置
- **连接状态指示** — 显示已连接/扫描中/断开及蓝牙异常状态
- **HTTP API 服务** — 支持 REST 和 SSE 实时推送，便于浏览器或多客户端接入
- **自动重连** — 设备断开后自动扫描并重新连接，指数退避
- **跨平台支持** — Windows、MacOS、Linux

---

## 🖥️ 原生界面说明

本程序主界面为 Slint 原生渲染窗口，提供高效的实时心率监控体验：

- 大号心率数字实时显示
- 心率区间彩色标签（热身/燃脂/有氧/极限）
- 最低/最高/平均统计面板
- 重置统计按钮
- 无需浏览器或WebView，降低内存和CPU占用

---

## 📺 直播叠加层说明

内置的 HTTP 服务器提供以下接口，便于集成：

| 接口 | 说明 |
|------|------|
| GET /heart-rate | 获取当前心率 JSON |
| GET /heart-rate-stream | SSE 实时推送心率数据流 |
| GET /health | 健康检查与版本信息 |

默认监听地址：

```
http://127.0.0.1:3030
```

（若端口冲突则自动使用随机端口，启动时会有日志提示）

通过浏览器或 OBS 浏览器源访问此地址即可展示心率数据，支持定制前端样式。

---

## 📦 快速开始

### 下载构建产物

1. 前往 [GitHub Releases 页面](https://github.com/Roxy-0304/band-heart-rate/releases)
2. 下载最新版本对应的可执行文件（例如 Windows 下为 `band-heart-rate.exe`）
3. 直接运行即可

### 从源码编译

**环境要求：**

- [Rust 工具链](https://www.rust-lang.org/tools/install)（推荐使用 rustup 安装）
- [Slint 工具链](https://slint.dev/)（用于构建原生 UI）
- Windows 环境需开启蓝牙支持

**编译步骤：**

```bash
# 克隆仓库
git clone https://github.com/Roxy-0304/band-heart-rate.git
cd band-heart-rate

# 编译默认带原生 GUI 的版本
cargo build --release

# 或编译无 GUI 的纯后端版本（仅 HTTP API）
cargo build --release --no-default-features --features=""
```

---

## 🚀 使用指南

1. 在**手环/手表设置**中开启 **心率广播** 功能
2. 确保设备蓝牙已开启
3. 运行编译后的可执行文件
4. 程序会自动扫描周围的心率广播设备并连接
5. 心率数据将在原生窗口实时显示，同时提供 HTTP API 供浏览器或其他客户端访问
6. 可通过浏览器访问 `http://127.0.0.1:3030/heart-rate-stream` 查看实时数据流

---

## ⚙️ 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `MIBAND_ALLOWED_DEVICES` | 允许连接的设备名称关键词（逗号分隔，不区分大小写） | `band,amazfit,watch,mi` |

**示例：**

```bash
# 仅允许连接包含 "mi" 或 "honor" 的设备
set MIBAND_ALLOWED_DEVICES=mi,honor
band-heart-rate.exe
```

---

## 📁 项目结构

```
band-heart-rate/
├── src/
│   ├── main.rs              # 程序入口及初始化
│   ├── gui.rs               # Slint 原生 GUI 窗口逻辑
│   ├── ble.rs               # 蓝牙 BLE 心率设备通信
│   ├── server.rs            # HTTP API 服务器（REST 和 SSE）
│   ├── types.rs             # 共享数据结构定义
│   └── macros.rs            # 辅助宏定义
├── ui/
│   └── app.slint            # Slint UI 界面布局定义
├── doc/
│   ├── 1.png                # 桌面主界面截图
│   ├── 2.gif                # 运行演示动图
│   └── 3.png                # 桌面功能展示截图
├── icons/
│   └── icon.ico             # 应用图标
├── build.rs                 # 构建脚本
├── Cargo.toml               # Rust 项目配置与依赖
├── Cargo.lock               # 依赖锁定文件
├── README.md                # 中文文档
├── README_EN.md             # 英文文档
└── LICENSE                  # 开源许可证
```

---

## 🖼️ 截图

![桌面主界面](doc/1.png)

![桌面功能展示](doc/3.png)

![运行演示](doc/2.gif)

---

## 支持的平台

基于 Slint 框架和 `bluest` 蓝牙库，支持：

- Windows 10/11
- MacOS
- Linux

---

## 兼容设备

兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备，包括但不限于：

- **小米手环**系列（已在 小米手环 10 上测试通过）
- **荣耀手环**系列
- **华为手环/手表**系列
- **Amazfit** 品牌设备
- **Apple Watch**
- 其他支持 BLE 心率广播的运动手表/胸带等

> 如果设备支持“心率广播”功能，在设备设置中开启后即可被本程序识别。
