use serde::Serialize;
use std::error::Error;
use std::fmt;
use tokio::sync::watch;

/// 设备连接/通信过程中发生的可重连错误类型
pub enum ReconnectError {
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

#[derive(Clone, Default, Serialize)]
pub struct HeartRateReading {
    pub heart_rate: u16,
    pub sensor_contact: Option<bool>,
    pub connected: bool,
    pub scanning: bool,
    /// 蓝牙任务退出时填充错误信息，前端可据此显示提示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub rx: watch::Receiver<HeartRateReading>,
}
