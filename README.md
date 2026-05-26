[English](README_EN.md) 或 [中文](README.md)

## 免责声明

> 本项目 Fork 自 [Tnze/miband-heart-rate](https://github.com/Tnze/miband-heart-rate)，代码由 AI 编写。

# BLE 心率监测演示程序

通过标准 BLE 心率服务（Heart Rate Service, UUID 0x180D）接收心率广播数据的演示程序。需要在手环/手表的设置中开启心率广播功能。

欢迎二次开发。

## 支持的平台

使用 `bluest` crate。以下引用其说明：

> Bluest 是一个跨平台的 Rust 低功耗蓝牙（BLE）库。目前支持 Windows（版本 10 及以上）、MacOS/iOS 和 Linux。Android 支持正在计划中。

因此支持：

- Windows 10/11
- MacOS/iOS
- Linux

## 兼容设备

本项目兼容任何支持标准 BLE 心率服务（UUID 0x180D）的穿戴设备，包括但不限于：

- **小米手环**系列（已在 小米手环 10 上测试通过）
- **荣耀手环**系列
- **华为手环/手表**系列
- **Amazfit** 品牌设备
- **Apple Watch**
- 其他支持 BLE 心率广播的运动手表/胸带等

> 如果设备支持"心率广播"功能，在设备设置中开启后即可被本程序识别。
>
> 可通过环境变量 `MIBAND_ALLOWED_DEVICES` 自定义允许连接的设备名称关键词（逗号分隔），默认关键词为：`band`、`amazfit`、`watch`。

## 功能特性

- **实时心率显示**：使用 `print!` + `flush()` 在同一行实时刷新
- **Web 界面**：在浏览器中实时显示心率数据和传感器接触状态
- **自定义样式**：支持在 Web 界面中注入自定义 CSS
- **跨平台支持**：Windows、macOS/iOS、Linux

### 访问 Web 界面

在浏览器中打开：`http://127.0.0.1:3030`

端口号会在程序启动时显示，默认是3030，如果有冲突会自动切换到其他端口。

实时查看心率数据和传感器状态。

## 提示
默认的窗口大小为1920x1080，如果显示不完整，请调整窗口大小。

## 截图

![Alt text](doc/1.png)
![Alt text](doc/2.gif)
