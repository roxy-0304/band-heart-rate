use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

use axum::{extract::State, response::Html, routing::get, Json, Router};

/// 打印并立即刷新 stdout 的宏
macro_rules! printfl {
    ($($arg:tt)*) => {{
        print!($($arg)*);
        std::io::stdout().flush().unwrap();
    }};
}

/// 打印并立即刷新 stderr 的宏
macro_rules! eprintfl {
    ($($arg:tt)*) => {{
        eprint!($($arg)*);
        std::io::stderr().flush().unwrap();
    }};
}

use bluest::{btuuid::bluetooth_uuid_from_u16, Adapter, Device, Uuid};
use futures_lite::stream::StreamExt;
use serde::Serialize;
use tokio::sync::watch;
use tokio::signal;
use tokio::time::{timeout, Duration};

const HRS_UUID: Uuid = bluetooth_uuid_from_u16(0x180D);
const HRM_UUID: Uuid = bluetooth_uuid_from_u16(0x2A37);

/// 扫描总超时：2 分钟无设备则退出
const SCAN_TOTAL_TIMEOUT_SECS: u64 = 120;
/// 单次扫描超时：最多 10 秒（比原来 30 秒快 3 倍）
const SCAN_ATTEMPT_TIMEOUT_SECS: u64 = 10;
/// 首次连接后等待第一个数据的超时时间
const INITIAL_DATA_TIMEOUT_SECS: u64 = 30;
/// 正常运行时两个数据包之间的超时时间
const NORMAL_DATA_TIMEOUT_SECS: u64 = 5;
/// 重连间隔
const RECONNECT_DELAY_SECS: u64 = 2;
/// 扫描失败后重试间隔
const SCAN_RETRY_DELAY_SECS: u64 = 1;

/// 默认允许连接的设备名称关键词（不区分大小写，名称包含任意一个即匹配）
const DEFAULT_ALLOWED_KEYWORDS: &[&str] = &[
    "band",
    "amazfit",
    "watch",
];

/// 读取允许的设备名称关键词（优先读取环境变量 MIBAND_ALLOWED_DEVICES，逗号分隔；否则使用默认列表）
/// 结果通过 OnceLock 缓存，只在首次调用时解析一次
fn allowed_keywords() -> &'static [String] {
    static CACHE: OnceLock<Vec<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        match std::env::var("MIBAND_ALLOWED_DEVICES") {
            Ok(val) => {
                let keywords: Vec<String> = val.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                if keywords.is_empty() {
                    // 环境变量为空或只有逗号分隔的空段时回退到默认
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
/// 名称未知时（None）也允许通过（靠 UUID 过滤 + known_ids 兜底），
/// 避免因扫描初期未解析出名称而漏掉设备
fn device_name_allowed(name: Option<&str>, keywords: &[String]) -> bool {
    match name {
        Some(name) => {
            let name_lower = name.to_lowercase();
            keywords.iter().any(|kw| name_lower.contains(kw.as_str()))
        }
        None => true,
    }
}

/// 判断设备是否应该加入候选列表：
/// 1. 名称匹配关键词；或
/// 2. 名称未知（None）；或
/// 3. 以前成功连接过（known_ids 中有记录）
fn device_should_try(
    name: Option<&str>,
    device_id: &str,
    keywords: &[String],
    known_ids: &HashSet<String>,
) -> bool {
    if known_ids.contains(device_id) {
        return true;
    }
    device_name_allowed(name, keywords)
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

#[derive(Clone, Copy, Serialize)]
struct HeartRateReading {
    heart_rate: u16,
    sensor_contact: Option<bool>,
    connected: bool,
    scanning: bool,
}

impl Default for HeartRateReading {
    fn default() -> Self {
        Self {
            heart_rate: 0,
            sensor_contact: None,
            connected: false,
            scanning: false,
        }
    }
}

#[derive(Clone)]
struct AppState {
    rx: watch::Receiver<HeartRateReading>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = watch::channel(HeartRateReading::default());

    tokio::spawn(async move {
        if let Err(err) = run_server(rx).await {
            eprintfl!("\nWeb server error: {err}\n");
        }
    });

    let adapter = Adapter::default()
        .await
        .ok_or("Bluetooth adapter not found")?;
    adapter.wait_available().await?;

    tokio::select! {
        _ = signal::ctrl_c() => {
            printfl!("Received shutdown signal, exiting...\n");
        }
        result = run_loop(adapter, tx) => {
            if let Err(e) = result {
                eprintfl!("\nLoop error: {e}\n");
            }
        }
    }

    Ok(())
}

async fn run_loop(
    adapter: Adapter,
    tx: watch::Sender<HeartRateReading>,
) -> Result<(), Box<dyn Error>> {
    let mut disconnect_time: Option<Instant> = None;
    // 记录曾经成功连接过的设备 ID，方便断线后快速重连
    let mut known_ids: HashSet<String> = HashSet::new();

    loop {
        // Check if we've been disconnected for too long
        if let Some(time) = disconnect_time {
            let elapsed = time.elapsed().as_secs();
            if elapsed >= SCAN_TOTAL_TIMEOUT_SECS {
                eprintfl!("\nScan timeout: No device found in 2 minutes, exiting...\n");
                return Err("Scan timeout: No device found in 2 minutes".into());
            }
        }

        // Update state to show we're scanning
        tx.send_replace(HeartRateReading { scanning: true, ..Default::default() });

        // Collect all devices discovered during the scan window
        let devices = match scan_all_devices(&adapter, tx.clone(), &known_ids).await {
            Ok(devices) => devices,
            Err(err) => {
                printfl!("Scan failed: {:?}\n", err);
                tx.send_replace(HeartRateReading::default());
                tokio::time::sleep(Duration::from_secs(SCAN_RETRY_DELAY_SECS)).await;
                continue;
            }
        };

        // Try each device until one succeeds
        let mut ever_connected = false;
        for device in &devices {
            match handle_device(&adapter, device, tx.clone()).await {
                Ok(()) => {
                    // Device connected and exited normally
                    known_ids.insert(device.id().to_string());
                    ever_connected = true;
                    break;
                }
                Err(err) if err.downcast_ref::<ReconnectError>().is_some() => {
                    // Device was connected and then lost — keep it in known_ids
                    known_ids.insert(device.id().to_string());
                    ever_connected = true;
                    continue;
                }
                Err(err) => {
                    // Failed to connect (e.g., busy/occupied)
                    printfl!("Cannot connect to this device: {err:?}, trying next...\n");
                    continue;
                }
            }
        }

        // All devices exhausted
        tx.send_replace(HeartRateReading::default());
        disconnect_time = Some(Instant::now());

        if ever_connected {
            tokio::time::sleep(Duration::from_secs(RECONNECT_DELAY_SECS)).await;
        } else {
            // Never connected — enter light polling
            printfl!("All devices busy or unreachable, entering light polling mode...\n");

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
) -> Result<(), Box<dyn Error>> {
    let deadline = Duration::from_secs(SCAN_TOTAL_TIMEOUT_SECS);
    let poll_duration = Duration::from_secs(5);
    let start = Instant::now();

    loop {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            return Err("Poll timeout: All devices busy, exiting...".into());
        }

        printfl!("Polling for available devices ({}s left)...\n",
            deadline.as_secs().saturating_sub(elapsed.as_secs()));

        tx.send_replace(HeartRateReading { scanning: true, ..Default::default() });

        let keywords = allowed_keywords();

        let mut scan = match adapter.discover_devices(&[HRS_UUID]).await {
            Ok(s) => s,
            Err(e) => {
                eprintfl!("Poll scan error: {e:?}\n");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let mut devices: HashSet<Device> = HashSet::new();
        let _ = timeout(poll_duration, async {
            while let Some(device_result) = scan.next().await {
                if let Ok(device) = device_result {
                    let name = device.name_async().await;
                    let id = device.id().to_string();
                    if device_should_try(name.as_deref().ok(), &id, &keywords, known_ids) {
                        devices.insert(device);
                    } else {
                        printfl!("Poll: skipping device [{}] {:?}: not in allowed keywords\n", id, name);
                    }
                }
            }
        }).await;

        tx.send_replace(HeartRateReading::default());

        if devices.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        for device in &devices {
            printfl!("Poll: trying device [{}]...\n", device.id());
            match handle_device(adapter, device, tx.clone()).await {
                Ok(()) => return Ok(()),
                Err(err) if err.downcast_ref::<ReconnectError>().is_some() => {
                    return Ok(());
                }
                Err(err) => {
                    printfl!("Poll: device still busy: {err:?}, cleaning up and trying next...\n");
                    let _ = adapter.disconnect_device(device).await;
                    continue;
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// 扫描并收集所有发现的设备（最多 SCAN_ATTEMPT_TIMEOUT_SECS 秒）。
/// 与旧版不同：**一旦发现第一个匹配的设备就立即返回**，不再干等 30 秒。
async fn scan_all_devices(
    adapter: &Adapter,
    tx: watch::Sender<HeartRateReading>,
    known_ids: &HashSet<String>,
) -> Result<Vec<Device>, Box<dyn Error>> {
    printfl!("Starting scan (collecting all devices)...\n");

    let keywords = allowed_keywords();
    printfl!("Allowed device keywords: {:?}\n", keywords);
    if !known_ids.is_empty() {
        printfl!("Known device IDs: {:?}\n", known_ids);
    }

    tx.send_replace(HeartRateReading { scanning: true, ..Default::default() });

    let mut scan = adapter.discover_devices(&[HRS_UUID]).await?;
    printfl!("Scan started\n");

    // 用 HashSet 去重，同一个手环的多次广播不会重复添加
    let mut devices: HashSet<Device> = HashSet::new();

    // 扫描最多 SCAN_ATTEMPT_TIMEOUT_SECS 秒，但第一个设备出现后等待 2 秒收集备选就返回
    let deadline = Duration::from_secs(SCAN_ATTEMPT_TIMEOUT_SECS);
    let settle_duration = Duration::from_secs(2); // 找到第一个后，再等 2 秒收集备选

    let _ = timeout(deadline, async {
        while let Some(device_result) = scan.next().await {
            match device_result {
                Ok(device) => {
                    let name = device.name_async().await;
                    let did = device.id().to_string();
                    let id_for_filter = did.clone();
                    if device_should_try(name.as_deref().ok(), &id_for_filter, &keywords, known_ids) {
                        let is_new = devices.insert(device);
                        if is_new {
                            printfl!("Found Device: [{}] {:?}\n", did, name);
                        }
                        if devices.len() == 1 {
                            break; // 找到第一个新设备，进入 settle 阶段
                        }
                    } else {
                        printfl!("Skipping device [{}] {:?}: not in allowed keywords\n", did, name);
                    }
                }
                Err(e) => {
                    eprintfl!("Scan error: {e:?}\n");
                }
            }
        }
    }).await;

    // 如果找到至少一个设备，再等最多 2 秒收集更多备选（已去重）
    if !devices.is_empty() {
        printfl!("First device found, collecting more candidates for 2 seconds...\n");
        let _ = timeout(settle_duration, async {
            while let Some(device_result) = scan.next().await {
                if let Ok(device) = device_result {
                    let name = device.name_async().await;
                    let id = device.id().to_string();
                    if device_should_try(name.as_deref().ok(), &id, &keywords, known_ids) {
                        if devices.insert(device) {
                            printfl!("Found additional Device: [{}] {:?}\n", id, name);
                        }
                    }
                }
            }
        }).await;
    }

    // 转回 Vec 供上层逻辑使用
    let devices: Vec<Device> = devices.into_iter().collect();

    tx.send_replace(HeartRateReading::default());

    if devices.is_empty() {
        Err("No devices found".into())
    } else {
        printfl!("Scan complete, found {} device(s)\n", devices.len());
        Ok(devices)
    }
}

async fn run_server(rx: watch::Receiver<HeartRateReading>) -> Result<(), Box<dyn Error>> {
    let app = Router::new()
        .route("/", get(index))
        .route("/heart-rate", get(heart_rate))
        .with_state(AppState { rx });

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:3030").await {
        Ok(l) => {
            printfl!("Serving web UI at http://127.0.0.1:3030/\n");
            l
        }
        Err(e) => {
            eprintfl!("Failed to bind to port 3030: {e}, trying random port...\n");
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let port = l.local_addr()?.port();
            printfl!("Serving web UI at http://127.0.0.1:{}/\n", port);
            l
        }
    };

    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <title>Mi Band Heart Rate</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link href="https://fonts.googleapis.com/css2?family=Orbitron:wght@700&display=swap" rel="stylesheet">
    <style>
        :root {
            --red: #FF3B30;
            --glow: rgba(255, 59, 48, 0.35);
            --dim: rgba(255, 59, 48, 0.25);
        }

        html, body {
            background: transparent !important;
            margin: 0;
            padding: 0;
            overflow: hidden;
            width: 1920px;
            height: 1080px;
        }

        body {
            display: flex;
            align-items: flex-end;
            justify-content: flex-start;
        }

        .container {
            display: flex;
            align-items: center;
            gap: 14px;
            margin-left: 60px;
            margin-bottom: 60px;
        }

        .heart {
            width: 90px;
            height: 90px;
            flex-shrink: 0;
            fill: var(--red);
            animation: pulse 1.2s ease-in-out infinite;
            filter: drop-shadow(0 0 12px var(--glow));
        }

        @keyframes pulse {
            0%   { transform: scale(1);    filter: drop-shadow(0 0 12px var(--glow)); }
            15%  { transform: scale(1.14); filter: drop-shadow(0 0 20px var(--glow)); }
            30%  { transform: scale(1);    filter: drop-shadow(0 0 12px var(--glow)); }
            45%  { transform: scale(1.07); filter: drop-shadow(0 0 16px var(--glow)); }
            60%  { transform: scale(1);    filter: drop-shadow(0 0 12px var(--glow)); }
        }

        .bpm-number {
            font-family: 'Orbitron', monospace;
            font-weight: 700;
            font-size: 100px;
            line-height: 1;
            color: var(--red);
            text-shadow: 0 0 30px rgba(255, 59, 48, 0.3);
            transition: color 0.4s ease;
        }

        .bpm-number.dim {
            color: var(--dim);
            text-shadow: none;
        }
    </style>
</head>
<body>
    <div class="container">
        <svg class="heart" viewBox="0 0 24 24">
            <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5
                     2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09
                     C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5
                     c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
        </svg>
        <div class="bpm-number" id="heart-rate">--</div>
    </div>

    <script>
        const el = document.getElementById('heart-rate');

        async function fetchRate() {
            try {
                const res = await fetch('/heart-rate');
                const data = await res.json();
                if (data.scanning || !data.connected || data.heart_rate == null) {
                    el.textContent = '--';
                    el.classList.add('dim');
                } else {
                    el.textContent = data.heart_rate;
                    el.classList.remove('dim');
                }
            } catch {
                el.textContent = '--';
                el.classList.add('dim');
            }
        }

        setInterval(fetchRate, 1000);
        fetchRate();
    </script>
</body>
</html>"#)
}

async fn heart_rate(State(state): State<AppState>) -> Json<HeartRateReading> {
    Json(*state.rx.borrow())
}

async fn handle_device(
    adapter: &Adapter,
    device: &Device,
    tx: watch::Sender<HeartRateReading>,
) -> Result<(), Box<dyn Error>> {
    printfl!("Attempting to connect to device: {}\n", device.id());

    // Connect — 即使 Windows 已自动连接，也先断开再重连，确保 bluest 能获取 HRS 服务访问权
    if device.is_connected().await {
        printfl!("Device already connected, disconnecting first...\n");
        let _ = adapter.disconnect_device(&device).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    adapter.connect_device(&device).await?;
    printfl!("Device connected successfully\n");

    // Discover services
    printfl!("Discovering services...\n");
    let heart_rate_services = device.discover_services_with_uuid(HRS_UUID).await?;
    let heart_rate_service = heart_rate_services
        .first()
        .ok_or("Device should has one heart rate service at least")?;

    // Discover characteristics
    printfl!("Discovering characteristics...\n");
    let heart_rate_measurements = heart_rate_service
        .discover_characteristics_with_uuid(HRM_UUID)
        .await?;
    let heart_rate_measurement = heart_rate_measurements
        .first()
        .ok_or("HeartRateService should has one heart rate measurement characteristic at least")?;

    printfl!("Setting up notifications...\n");
    let mut updates = heart_rate_measurement.notify().await?;

    // Send connected state
    tx.send_replace(HeartRateReading { connected: true, ..Default::default() });

    printfl!("Starting to receive heart rate data...\n");

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
            printfl!("\nNo heart rate data received for {} seconds, attempting to reconnect...\n", timeout_duration.as_secs());
            break;
        }

        // Use timeout to wait for next update
        match timeout(timeout_duration - elapsed, updates.next()).await {
            Ok(Some(Ok(heart_rate))) => {
                // 收到第一个数据后，标记为已接收
                if !first_data_received {
                    first_data_received = true;
                    printfl!("\nFirst heart rate data received, switching to normal timeout mode\n");
                }

                // Reset timeout timer on successful update
                last_update_time = Instant::now();

                let flag = *heart_rate.get(0).ok_or("No flag")?;

                // Heart Rate Value Format
                let mut heart_rate_value = *heart_rate.get(1).ok_or("No heart rate u8")? as u16;
                if flag & 0b00001 != 0 {
                    heart_rate_value |= (*heart_rate.get(2).ok_or("No heart rate u16")? as u16) << 8;
                }

                // Sensor Contact Supported
                let mut sensor_contact = None;
                if flag & 0b00100 != 0 {
                    sensor_contact = Some(flag & 0b00010 != 0)
                }

                tx.send_replace(HeartRateReading {
                    heart_rate: heart_rate_value,
                    sensor_contact,
                    connected: true,
                    scanning: false,
                });

                print!("\rHeartRateValue: {heart_rate_value}, SensorContactDetected: {sensor_contact:?}                    ");
                std::io::stdout().flush().unwrap();
            }
            Ok(Some(Err(e))) => {
                printfl!("\nNotification error: {:?}\n", e);
                break;
            }
            Ok(None) => {
                printfl!("\nHeart rate notifications stopped\n");
                break;
            }
            Err(_) => {
                // 理论上不应触发（elapsed ≥ timeout_duration 时已 break），
                // 但系统时钟调整等极端情况下仍可能进入，用 break 避免 panic
                break;
            }
        }
    }

    // 先判断设备连接状态，再断开
    let was_connected = device.is_connected().await;
    tx.send_replace(HeartRateReading::default());
    let _ = adapter.disconnect_device(&device).await;

    if was_connected {
        printfl!("Device stopped broadcasting, attempting to reconnect...\n");
        Err(Box::new(ReconnectError::StoppedBroadcasting))
    } else {
        printfl!("Device disconnected, attempting to reconnect...\n");
        Err(Box::new(ReconnectError::Disconnected))
    }
}
