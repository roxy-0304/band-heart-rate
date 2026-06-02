#![windows_subsystem = "windows"]

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

use axum::{extract::State, response::Html, routing::get, Json, Router};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

/// 打印并立即刷新 stdout 的宏（自动换行）
macro_rules! printfl {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().unwrap();
    }};
}

/// 打印并立即刷新 stderr 的宏（自动换行）
macro_rules! eprintfl {
    ($($arg:tt)*) => {{
        eprint!($($arg)*);
        std::io::stderr().flush().unwrap();
    }};
}

/// 不换行打印并立即刷新 stdout，用回车覆盖当前行
macro_rules! printfl_inline {
    ($($arg:tt)*) => {{
        print!("\r");
        print!($($arg)*);
        std::io::stdout().flush().unwrap();
    }};
}

use bluest::{btuuid::bluetooth_uuid_from_u16, Adapter, Device, Uuid};
use futures_lite::stream::StreamExt;
use serde::Serialize;
use tokio::sync::watch;
use tokio::time::{timeout, Duration};

const HRS_UUID: Uuid = bluetooth_uuid_from_u16(0x180D);
const HRM_UUID: Uuid = bluetooth_uuid_from_u16(0x2A37);

/// 扫描总超时：2 分钟无设备则退出
const SCAN_TOTAL_TIMEOUT_SECS: u64 = 120;
/// 单次扫描超时：最多 10 秒
const SCAN_ATTEMPT_TIMEOUT_SECS: u64 = 10;
/// 首次连接后等待第一个数据的超时时间
const INITIAL_DATA_TIMEOUT_SECS: u64 = 30;
/// 正常运行时两个数据包之间的超时时间
const NORMAL_DATA_TIMEOUT_SECS: u64 = 25;
/// 重连退避最大值（秒）
const RECONNECT_BACKOFF_MAX_SECS: u64 = 30;
/// 扫描失败后重试间隔
const SCAN_RETRY_DELAY_SECS: u64 = 1;

/// 默认允许连接的设备名称关键词（不区分大小写，名称包含任意一个即匹配）
const DEFAULT_ALLOWED_KEYWORDS: &[&str] = &[
    "band",
    "amazfit",
    "watch",
    "mi",
];

/// 读取允许的设备名称关键词（优先读取环境变量 MIBAND_ALLOWED_DEVICES，逗号分隔；否则使用默认列表）
/// 结果通过 OnceLock 缓存，只在首次调用时解析一次
fn allowed_keywords() -> &'static [String] {
    static KEYWORDS: OnceLock<Vec<String>> = OnceLock::new();
    KEYWORDS.get_or_init(|| {
        match std::env::var("MIBAND_ALLOWED_DEVICES") {
            Ok(val) => {
                let keywords: Vec<String> = val.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if keywords.is_empty() {
                    DEFAULT_ALLOWED_KEYWORDS.iter().map(|s| s.to_lowercase()).collect()
                } else {
                    keywords
                }
            }
            Err(_) => {
                DEFAULT_ALLOWED_KEYWORDS.iter().map(|s| s.to_lowercase()).collect()
            }
        }
    })
}

/// 检查设备名称是否匹配允许的关键词列表
fn device_name_allowed(name: Option<&str>, keywords: &[String]) -> bool {
    match name {
        Some(name) => {
            let name_lower = name.to_lowercase();
            keywords.iter().any(|kw| name_lower.contains(kw.as_str()))
        }
        None => true,
    }
}

/// 设备连接/通信过程中发生的可重连错误类型
enum ReconnectError {
    /// 设备已物理断开连接
    Disconnected,
    /// 设备仍连接但停止广播数据（如手环退出心率模式）
    StoppedBroadcasting,
}

impl fmt::Display for ReconnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReconnectError::Disconnected => write!(f, "Device disconnected"),
            ReconnectError::StoppedBroadcasting => write!(f, "Device stopped broadcasting"),
        }
    }
}

impl fmt::Debug for ReconnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ReconnectError {}

#[derive(Clone, Serialize)]
struct HeartRateReading {
    heart_rate: u16,
    sensor_contact: Option<bool>,
    connected: bool,
    scanning: bool,
    /// 蓝牙任务退出时填充错误信息，前端可据此显示提示
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Default for HeartRateReading {
    fn default() -> Self {
        Self {
            heart_rate: 0,
            sensor_contact: None,
            connected: false,
            scanning: false,
            error: None,
        }
    }
}

#[derive(Clone)]
struct AppState {
    rx: watch::Receiver<HeartRateReading>,
}

/// Holds the latest reading for the `get-latest-reading` Tauri command.
#[derive(Clone)]
struct LatestReading(Arc<Mutex<HeartRateReading>>);

// ===== Tauri Commands =====

#[tauri::command]
fn get_latest_reading(state: tauri::State<'_, LatestReading>) -> HeartRateReading {
    let guard = state.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    guard.clone()
}

/// 为窗口添加关闭按钮 guard：第一次 CloseRequested 时 prevent + close，
/// 第二次（由 close() 触发）放行。
fn install_close_guard(window: &tauri::WebviewWindow) {
    let w = window.clone();
    let close_guard = Arc::new(AtomicBool::new(false));
    let close_guard_1 = close_guard.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            if !close_guard_1.load(Ordering::SeqCst) {
                api.prevent_close();
                close_guard_1.store(true, Ordering::SeqCst);
                let _ = w.close();
            }
        }
    });
}

// ===== Tauri Application Entry =====

fn main() {
    // Flag: true only when tray "quit" is clicked; lets ExitRequested through
    let quit_flag = Arc::new(AtomicBool::new(false));

    // Clone for tray closure so original can be moved into `.run()` closure
    let quit_flag_tray = quit_flag.clone();

    tauri::Builder::default()
        .setup(|app| {
            let (tx, rx) = watch::channel(HeartRateReading::default());

            // Clone a receiver for the Tauri event emitter
            let mut rx_tauri = rx.clone();

            // Clone app handle for use in spawned tasks
            let app_handle = app.handle().clone();

            // Shared state for Tauri command (catch-up on window show)
            let latest_reading = LatestReading(Arc::new(Mutex::new(HeartRateReading::default())));
            app.manage(latest_reading.clone());

            // --- Spawn axum Web server (OBS UI, unchanged) ---
            tauri::async_runtime::spawn(async move {
                if let Err(err) = run_server(rx).await {
                    eprintfl!("\nWeb 服务器错误: {err}\n");
                }
            });

            // --- Spawn Tauri event emitter: only forward to frontend when window is visible ---
            let latest_1 = latest_reading.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    if rx_tauri.changed().await.is_ok() {
                        let reading = rx_tauri.borrow().clone();
                        // Always keep latest reading for catch-up
                        if let Ok(mut latest) = latest_1.0.lock() {
                            *latest = reading.clone();
                        }
                        // Only emit to frontend if window exists AND is visible
                        // (window is destroyed when minimised to tray — saves ~60-100MB)
                        if let Some(win) = app_handle.get_webview_window("main") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = app_handle.emit("hr-update", reading);
                            }
                        }
                    } else {
                        break;
                    }
                }
            });

            // --- 托盘图标：关闭 → 最小化到托盘，右键退出 ---
            let _ = tauri::tray::TrayIconBuilder::new()
                .menu(&tauri::menu::MenuBuilder::new(app)
                    .item(&tauri::menu::MenuItemBuilder::new("显示窗口").id("show").build(app)?)
                    .separator()
                    .item(&tauri::menu::MenuItemBuilder::new("退出").id("quit").build(app)?)
                    .build()?)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                        } else {
                            // Window was destroyed (closed to tray) — recreate it
                            if let Ok(new_win) = tauri::WebviewWindowBuilder::new(
                                app,
                                "main",
                                tauri::WebviewUrl::App("index.html".into()),
                            )
                            .title("Band Heart Rate Monitor")
                            .inner_size(600.0, 750.0)
                            .min_inner_size(480.0, 600.0)
                            .resizable(true)
                            .center()
                            .build()
                            {
                                install_close_guard(&new_win);
                            }
                        }
                    }
                    "quit" => {
                        quit_flag_tray.store(true, Ordering::SeqCst);
                        app.exit(0);
                    }
                    _ => {}
                })
                .icon(app.default_window_icon().cloned().unwrap())
                .build(app)?;

            // --- 关闭按钮 → 直接销毁窗口（释放 ~60-100MB WebView 内存）---
            let window = app.get_webview_window("main").unwrap();
            install_close_guard(&window);

            // --- Spawn Bluetooth loop in a separate task ---
            tauri::async_runtime::spawn(async move {
                let adapter = match Adapter::default().await {
                    Some(a) => a,
                    None => {
                        printfl!("⚠ Bluetooth 适配器未找到（系统无蓝牙或驱动异常）\n");
                        tx.send_replace(HeartRateReading {
                            error: Some("蓝牙适配器未找到".into()),
                            ..Default::default()
                        });
                        return;
                    }
                };

                // wait_available 可能在某些蓝牙栈异常状态下永远不返回，加 5 秒超时
                match tokio::time::timeout(Duration::from_secs(5), adapter.wait_available()).await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        printfl!("⚠ Bluetooth 适配器不可用: {e}\n");
                        tx.send_replace(HeartRateReading {
                            error: Some(format!("蓝牙适配器不可用: {e}")),
                            ..Default::default()
                        });
                        return;
                    }
                    Err(_) => {
                        printfl!("⚠ Bluetooth 适配器无响应（5 秒超时），请检查蓝牙是否开启\n");
                        tx.send_replace(HeartRateReading {
                            error: Some("蓝牙适配器无响应，请检查蓝牙是否开启".into()),
                            ..Default::default()
                        });
                        return;
                    }
                }

                if let Err(e) = run_loop(adapter, tx.clone()).await {
                    eprintfl!("蓝牙循环退出: {e}\n");
                    tx.send_replace(HeartRateReading {
                        error: Some(format!("蓝牙服务已停止: {e}")),
                        ..Default::default()
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_latest_reading])
        .build(tauri::generate_context!())
        .expect("启动 Tauri 应用失败")
        .run(move |_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !quit_flag.load(Ordering::SeqCst) {
                    // User clicked X → window is destroyed but don't exit the app.
                    // Keep tray running with Bluetooth/Web server in background.
                    api.prevent_exit();
                }
                // If quit_flag is true → tray "quit" was clicked → let it exit.
            }
        });
}

async fn run_server(rx: watch::Receiver<HeartRateReading>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let app = Router::new()
        .route("/", get(index))
        .route("/heart-rate", get(heart_rate))
        .with_state(AppState { rx });

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:3030").await {
        Ok(l) => {
            printfl!("Web UI 运行在 http://127.0.0.1:3030/\n");
            l
        }
        Err(e) => {
            eprintfl!("端口 3030 绑定失败: {e}，尝试随机端口...\n");
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let port = l.local_addr()?.port();
            printfl!("Web UI 运行在 http://127.0.0.1:{}/\n", port);
            l
        }
    };

    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../web-ui.html"))
}

async fn heart_rate(State(state): State<AppState>) -> Json<HeartRateReading> {
    Json(state.rx.borrow().clone())
}

// ===== Bluetooth logic =====

async fn run_loop(
    adapter: Adapter,
    tx: watch::Sender<HeartRateReading>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut disconnect_time: Option<Instant> = None;
    // 记录曾经成功连接过的设备 ID，方便断线后快速重连
    let mut known_ids: HashSet<String> = HashSet::new();
    // 重连尝试计数（用于指数退避）
    let mut reconnect_attempts: u32 = 0;
    // 是否曾经成功连接过（用于区分首次启动和重连）
    let mut has_ever_connected = false;

    loop {
        // Check if we've been disconnected for too long
        if let Some(time) = disconnect_time {
            let elapsed = time.elapsed().as_secs();
            if elapsed >= SCAN_TOTAL_TIMEOUT_SECS {
                printfl_inline!("[✗] 超时退出：{} 秒内未找到设备\n", SCAN_TOTAL_TIMEOUT_SECS);
                return Err("扫描超时：120 秒内未找到任何设备".into());
            }
        }

        // 是否处于重连模式（已连接过但断开）
        let is_reconnecting = has_ever_connected && disconnect_time.is_some();

        // Update state to show we're scanning
        tx.send_replace(HeartRateReading { scanning: true, ..Default::default() });

        if is_reconnecting {
            reconnect_attempts += 1;
            printfl_inline!("[重连 #{reconnect_attempts}] 正在扫描设备...");
        } else {
            reconnect_attempts = 0;
            printfl!("正在扫描设备...\n");
        }

        // Collect all devices discovered during the scan window
        let devices = match scan_all_devices(&adapter, tx.clone(), is_reconnecting).await {
            Ok(devices) => devices,
            Err(err) => {
                if is_reconnecting {
                    printfl_inline!("[重连 #{reconnect_attempts}] 扫描失败，等待重试...");
                } else {
                    printfl!("扫描失败: {:?}\n", err);
                }
                tx.send_replace(HeartRateReading::default());
                tokio::time::sleep(Duration::from_secs(SCAN_RETRY_DELAY_SECS)).await;
                continue;
            }
        };

        // Try each device — known_ids first, then keyword-matched
        let keywords = allowed_keywords();
        // 先连接已知设备（之前成功连过的），再试其他关键词匹配的设备
        let (known_devices, other_devices): (Vec<&Device>, Vec<&Device>) = devices
            .iter()
            .partition(|d| known_ids.contains(&d.id().to_string()));

        for device in known_devices.iter().chain(other_devices.iter()) {
            let device_name = device.name_async().await.ok();
            if !known_ids.contains(&device.id().to_string()) && !device_name_allowed(device_name.as_deref(), keywords) {
                if !is_reconnecting {
                    printfl!("跳过设备 [{}] {:?}: 不在允许关键词列表中\n", device.id(), device_name);
                }
                continue;
            }
            if is_reconnecting {
                printfl_inline!("[重连 #{reconnect_attempts}] 正在连接设备 {}...", device.id());
            }
            // handle_device 现在只在设备物理断开或停止广播时返回 Err(ReconnectError)，
            // 不会返回 Ok(())（因为通知循环结束后总是断开返回 Err）。
            // 无论 Err 是 ReconnectError 还是其他错误，我们都统一处理：
            // - ReconnectError → 记录 known_id, 设置 has_ever_connected 和 disconnect_time, 继续下一个
            // - 其他错误 → 简单继续下一个
            match handle_device(&adapter, device, tx.clone()).await {
                Ok(()) => {
                    // handle_device 目前永远不会返回 Ok, 此分支保留以备今后扩展
                    known_ids.insert(device.id().to_string());
                    has_ever_connected = true;
                    disconnect_time = None;
                    break;
                }
                Err(err) => {
                    // 只要是 ReconnectError（断开或停止广播），说明曾经连接成功过
                    if err.downcast_ref::<ReconnectError>().is_some() {
                        known_ids.insert(device.id().to_string());
                        has_ever_connected = true;
                        disconnect_time = Some(Instant::now());
                        continue;
                    }
                    if !is_reconnecting {
                        printfl!("无法连接该设备: {err:?}，尝试下一个...\n");
                    }
                    continue;
                }
            }
        }

        // All devices exhausted
        tx.send_replace(HeartRateReading::default());
        disconnect_time = disconnect_time.or(Some(Instant::now()));

        if has_ever_connected {
            // 无论本次是否成功连接过，都使用同一套退避逻辑
            let backoff_secs = std::cmp::min(
                1 << reconnect_attempts.min(10),
                RECONNECT_BACKOFF_MAX_SECS,
            );
            let detail = if devices.is_empty() { "未找到可连接设备" } else { "等待重试" };
            printfl_inline!("[重连 #{reconnect_attempts}] {detail} {backoff_secs} 秒后重扫...");
            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
        } else {
            // Never connected — enter light polling
            reconnect_attempts = 0;
            printfl!("所有设备忙碌或不可达，进入轻量轮询模式...\n");

            if let Err(err) = poll_for_available_device(&adapter, tx.clone(), &known_ids).await {
                return Err(err);
            }
            disconnect_time = None;
            continue;
        }
    }
}

/// 轻量轮询模式：全部设备被占时，用较短的扫描快速轮询是否有人释放。
/// 总超时由 SCAN_TOTAL_TIMEOUT_SECS 控制。
async fn poll_for_available_device(
    adapter: &Adapter,
    tx: watch::Sender<HeartRateReading>,
    known_ids: &HashSet<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let deadline = Duration::from_secs(SCAN_TOTAL_TIMEOUT_SECS);
    let poll_duration = Duration::from_secs(5);
    let start = Instant::now();
    let mut poll_count: u32 = 0;

    loop {
        poll_count += 1;
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            return Err("轮询超时：所有设备均忙碌，退出程序".into());
        }

        printfl_inline!("[轮询 #{poll_count}] 正在扫描可用设备（剩余 {} 秒）...",
            deadline.as_secs().saturating_sub(elapsed.as_secs()));

        tx.send_replace(HeartRateReading { scanning: true, ..Default::default() });

        let mut scan = match tokio::time::timeout(Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS), adapter.discover_devices(&[HRS_UUID])).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                printfl_inline!("[轮询 #{poll_count}] 扫描错误: {e:?}");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            Err(_) => {
                printfl_inline!("[轮询 #{poll_count}] 扫描启动超时");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let mut devices: Vec<Device> = Vec::new();
        let _ = timeout(poll_duration, async {
            while let Some(device_result) = scan.next().await {
                if let Ok(device) = device_result {
                    devices.push(device);
                }
            }
        }).await;

        tx.send_replace(HeartRateReading::default());

        if devices.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // 优先尝试已知设备，再试其他
        let keywords = allowed_keywords();
        let (known_devices, other_devices): (Vec<&Device>, Vec<&Device>) = devices
            .iter()
            .partition(|d| known_ids.contains(&d.id().to_string()));

        for device in known_devices.iter().chain(other_devices.iter()) {
            let device_name = device.name_async().await.ok();
            if !known_ids.contains(&device.id().to_string()) && !device_name_allowed(device_name.as_deref(), keywords) {
                continue;
            }
            printfl_inline!("[轮询 #{poll_count}] 尝试连接设备 {}...", device.id());
            let handled = handle_device(adapter, device, tx.clone()).await;
            if handled.is_ok() || handled.as_ref().err().and_then(|e| e.downcast_ref::<ReconnectError>()).is_some() {
                printfl!("\n[✓] 轮询成功，已连接设备 {}\n", device.id());
                return Ok(());
            }
            let _ = adapter.disconnect_device(device).await;
            continue;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// 扫描并收集所有发现的设备（最多 SCAN_ATTEMPT_TIMEOUT_SECS 秒）。
/// 一旦发现第一个匹配的设备就立即返回，再等 3 秒收集备选，不再干等 10 秒。
/// 重连模式下只输出简短的单行状态，不输出详细日志。
async fn scan_all_devices(
    adapter: &Adapter,
    tx: watch::Sender<HeartRateReading>,
    is_reconnecting: bool,
) -> Result<Vec<Device>, Box<dyn Error + Send + Sync>> {
    tx.send_replace(HeartRateReading { scanning: true, ..Default::default() });

    let mut scan = match tokio::time::timeout(Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS), adapter.discover_devices(&[HRS_UUID])).await {
        Ok(Ok(scan)) => scan,
        Ok(Err(e)) => return Err(format!("蓝牙扫描启动失败: {e}").into()),
        Err(_) => return Err("蓝牙扫描启动超时（10 秒无响应）".into()),
    };

    // 用 HashSet 去重，同一个手环的多次广播不会重复添加
    let mut devices_set: HashSet<Device> = HashSet::new();

    // 扫描最多 SCAN_ATTEMPT_TIMEOUT_SECS 秒，但第一个设备出现后等待 3 秒收集备选就返回
    let deadline = Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS);
    let settle_duration = Duration::from_secs(3);

    let _ = timeout(deadline, async {
        while let Some(device_result) = scan.next().await {
            match device_result {
                Ok(device) => {
                    let name = device.name_async().await;
                    let did = device.id().to_string();
                    let is_new = devices_set.insert(device);
                    if is_new && !is_reconnecting {
                        printfl!("发现设备: [{}] {:?}\n", did, name);
                    }
                    if devices_set.len() == 1 {
                        break;
                    }
                }
                Err(e) => {
                    if !is_reconnecting {
                        eprintfl!("扫描错误: {e:?}\n");
                    }
                }
            }
        }
    }).await;

    if !devices_set.is_empty() {
        if !is_reconnecting {
            printfl!("已找到首个设备，继续收集 3 秒备选...\n");
        }
        let _ = timeout(settle_duration, async {
            while let Some(device_result) = scan.next().await {
                if let Ok(device) = device_result {
                    let name = device.name_async().await;
                    let id = device.id().to_string();
                    if devices_set.insert(device) && !is_reconnecting {
                        printfl!("发现额外设备: [{}] {:?}\n", id, name);
                    }
                }
            }
        }).await;
    }

    let devices: Vec<Device> = devices_set.into_iter().collect();

    tx.send_replace(HeartRateReading::default());

    if devices.is_empty() {
        Err("未找到任何设备".into())
    } else {
        if !is_reconnecting {
            printfl!("扫描完成，发现 {} 个设备\n", devices.len());
        }
        Ok(devices)
    }
}

async fn handle_device(
    adapter: &Adapter,
    device: &Device,
    tx: watch::Sender<HeartRateReading>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    printfl!("正在连接设备: {}\n", device.id());

    // Connect — 先断开清理状态，再重连，确保 bluest 能获取 HRS 服务访问权
    let _ = tokio::time::timeout(Duration::from_secs(3), adapter.disconnect_device(&device)).await;
    adapter.connect_device(&device).await?;
    printfl!("设备连接成功\n");

    // Discover services
    printfl!("正在发现服务...\n");
    let heart_rate_services = device.discover_services_with_uuid(HRS_UUID).await?;
    let heart_rate_service = heart_rate_services
        .first()
        .ok_or("设备至少应包含一个心率服务")?;

    // Discover characteristics
    printfl!("正在发现特征...\n");
    let heart_rate_measurements = heart_rate_service
        .discover_characteristics_with_uuid(HRM_UUID)
        .await?;
    let heart_rate_measurement = heart_rate_measurements
        .first()
        .ok_or("心率服务至少应包含一个心率测量特征")?;

    printfl!("正在设置通知...\n");
    let mut updates = heart_rate_measurement.notify().await?;

    // Send connected state
    tx.send_replace(HeartRateReading { connected: true, ..Default::default() });

    printfl!("开始接收心率数据...\n");

    // Track the last update time for timeout detection
    let mut last_update_time = Instant::now();
    let mut first_data_received = false;
    let initial_timeout = Duration::from_secs(INITIAL_DATA_TIMEOUT_SECS);
    let normal_timeout = Duration::from_secs(NORMAL_DATA_TIMEOUT_SECS);

    loop {
        // 根据是否收到第一个数据选择超时时间
        let timeout_duration = if !first_data_received {
            initial_timeout
        } else {
            normal_timeout
        };

        // 检查 elapsed 是否已经超过 timeout_duration，防止减法下溢 panic
        let elapsed = last_update_time.elapsed();
        if elapsed >= timeout_duration {
            printfl!("\n已 {} 秒未收到心率数据，尝试重连...\n", timeout_duration.as_secs());
            break;
        }

        // 等待 BLE 数据（带超时）
        let result = tokio::time::timeout(timeout_duration - elapsed, updates.next()).await;
        match result {
            Ok(Some(Ok(heart_rate))) => {
                if !first_data_received {
                    first_data_received = true;
                    printfl!("\n收到首条心率数据，切换至正常超时模式\n");
                }

                last_update_time = Instant::now();

                let flag = *heart_rate.get(0).ok_or("无标志字节")?;

                let mut heart_rate_value = *heart_rate.get(1).ok_or("无心率 u8 数据")? as u16;
                if flag & 0b00001 != 0 {
                    heart_rate_value |= (*heart_rate.get(2).ok_or("无心率 u16 数据")? as u16) << 8;
                }

                let mut sensor_contact = None;
                if flag & 0b00100 != 0 {
                    sensor_contact = Some(flag & 0b00010 != 0)
                }

                tx.send_replace(HeartRateReading {
                    heart_rate: heart_rate_value,
                    sensor_contact,
                    connected: true,
                    scanning: false,
                    error: None,
                });

                print!("\r心率值: {heart_rate_value}, 传感器接触: {sensor_contact:?}                    ");
                std::io::stdout().flush().unwrap();
            }
            Ok(Some(Err(e))) => {
                printfl!("\n通知错误: {:?}\n", e);
                break;
            }
            Ok(None) => {
                printfl!("\n心率通知已停止\n");
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    let still_connected = match tokio::time::timeout(Duration::from_secs(3), device.is_connected()).await {
        Ok(true) => true,
        _ => false,
    };
    tx.send_replace(HeartRateReading::default());
    let _ = tokio::time::timeout(Duration::from_secs(3), adapter.disconnect_device(&device)).await;

    if still_connected {
        printfl!("设备已停止广播，尝试重连...\n");
        Err(Box::new(ReconnectError::StoppedBroadcasting))
    } else {
        printfl!("设备已断开，尝试重连...\n");
        Err(Box::new(ReconnectError::Disconnected))
    }
}
