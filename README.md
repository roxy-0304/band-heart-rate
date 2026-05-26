[English](README_EN.md) 或 [中文](README.md)


## ⚠️ 免责声明

> 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。
---

## 📋 目录

- [项目简介](#项目简介)
- [✨ 功能特性](#功能特性)
- [🖥️ Web UI 说明](#web-ui-说明)
  - [自定义样式](#自定义样式)
- [📺 OBS 直播叠加层设置](#obs-直播叠加层设置)
- [📦 快速开始](#快速开始)
  - [下载构建产物](#下载构建产物)
  - [从源码编译](#从源码编译)
- [🚀 使用指南](#使用指南)
- [⚙️ 环境变量](#环境变量)
- [📁 项目结构](#项目结构)
- [🖼️ 截图](#截图)
- [支持的平台](#支持的平台)
- [兼容设备](#兼容设备)

---

## 项目简介

BLE 心率监测演示程序，通过标准 BLE 心率服务（Heart Rate Service, UUID 0x180D）接收心率广播数据。需要在手环/手表的设置中开启心率广播功能。

> 💡 最新构建版本可从 [GitHub Releases](https://github.com/Roxy-0304/miband-heart-rate/releases) 页面下载。

---

## ✨ 功能特性

- **实时心率显示**：终端中使用 `print!` + `flush()` 在同一行实时刷新
- **Web 界面**：在浏览器中实时显示心率数据和传感器接触状态
- **自定义样式**：支持在 Web 界面中注入自定义 CSS
- **OBS 直播兼容**：页面专为直播叠加场景设计
- **自动重连**：设备断开后自动扫描并重新连接
- **跨平台支持**：Windows、macOS/iOS、Linux

---

## 🖥️ Web UI 说明

程序启动后会自动启动一个 Web 服务器，默认地址为：

```
http://127.0.0.1:3030
```

（如有端口冲突会自动切换到其他端口，启动时会有提示）

Web 页面特性：
- 设计尺寸为 **1920x1080**，适合全屏展示
- **透明背景**，无需绿幕即可直接叠加到直播画面
- 心跳图标带有 **脉冲动画** 效果
- 心率数字使用 **Orbitron 字体** 科技感显示
- 未连接时数字呈 **半透明** 状态

### 自定义样式

页面支持通过注入自定义 CSS 来修改外观。你可以在浏览器开发者工具中覆盖样式，或在 OBS 浏览器源的 CSS 选项中添加自定义代码。

示例修改颜色：
```css
:root {
  --red: #00FF00;   /* 改为绿色 */
}
```

---

## 📺 OBS 直播叠加层设置

本程序的 Web 界面专为直播叠加场景设计，可轻松集成到 OBS Studio 中。

**设置步骤：**

1. 运行程序，确保 Web 界面可访问
2. 在 OBS 中点击 `+` → **浏览器** 添加浏览器源
3. 在属性中设置：
   - **URL**：`http://127.0.0.1:3030`
   - **宽度**：`1920`
   - **高度**：`1080`
4. 点击确定完成添加

页面自带透明背景，无需额外抠图或绿幕设置。

---

## 📦 快速开始

### 下载构建产物

1. 前往 [GitHub Releases 页面](https://github.com/Roxy-0304/miband-heart-rate/releases)
2. 下载最新版本的 `miband-heart-rate.exe`
3. 直接运行即可

### 从源码编译

**环境要求：**

- [Rust 工具链](https://www.rust-lang.org/tools/install)（推荐使用 rustup 安装）

**编译步骤：**

```bash
# 克隆仓库
git clone https://github.com/Roxy-0304/miband-heart-rate.git
cd miband-heart-rate

# 编译发布版本
cargo build --release

# 可执行文件位于 target/release/miband-heart-rate.exe
```

---

## 🚀 使用指南

1. 在**手环/手表设置**中开启 **心率广播** 功能
2. 确保设备蓝牙已开启
3. 运行程序：
   ```bash
   # 如果是从源码编译
   cargo run --release

   # 或者直接运行下载的 exe
   ./miband-heart-rate.exe
   ```
4. 程序会自动扫描周围的心率广播设备并连接
5. 心率数据将在终端实时显示，同时可在浏览器访问 Web 界面

**提示：** 默认窗口大小为 1920x1080，如果显示不完整，请调整窗口大小。

---

## ⚙️ 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `MIBAND_ALLOWED_DEVICES` | 允许连接的设备名称关键词（逗号分隔，不区分大小写） | `band,amazfit,watch` |

**示例：**

```bash
# 仅允许连接包含 "mi" 或 "honor" 的设备
set MIBAND_ALLOWED_DEVICES=mi,honor
miband-heart-rate.exe
```

---

## 📁 项目结构

```
band-heart-rate/
├── src/
│   └── main.rs          # 主程序入口（蓝牙连接、Web 服务器、数据处理）
├── doc/
│   ├── 1.png            # 截图
│   └── 2.gif            # 动图
├── .github/
│   └── workflows/
│       └── release.yml  # GitHub Actions 自动构建发布
├── Cargo.toml           # 项目配置与依赖管理
├── Cargo.lock           # 依赖锁定文件
├── README.md            # 中文文档
├── README_EN.md         # 英文文档
└── LICENSE              # 开源许可证
```

---

## 🖼️ 截图

![Alt text](doc/1.png)
![Alt text](doc/2.gif)

---

## 支持的平台

使用 `bluest` crate。以下引用其说明：

> Bluest 是一个跨平台的 Rust 低功耗蓝牙（BLE）库。目前支持 Windows（版本 10 及以上）、MacOS/iOS 和 Linux。Android 支持正在计划中。

因此支持：

- Windows 10/11
- MacOS/iOS
- Linux

---

## 兼容设备

本项目兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备，包括但不限于：

- **小米手环**系列（已在 小米手环 10 上测试通过）
- **荣耀手环**系列
- **华为手环/手表**系列
- **Amazfit** 品牌设备
- **Apple Watch**
- 其他支持 BLE 心率广播的运动手表/胸带等

> 如果设备支持"心率广播"功能，在设备设置中开启后即可被本程序识别。
