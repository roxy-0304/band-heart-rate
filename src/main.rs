#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "gui")]
mod ble;
#[cfg(feature = "gui")]
mod macros;
#[cfg(feature = "gui")]
mod server;
#[cfg(feature = "gui")]
mod types;
#[cfg(feature = "gui")]
mod web_ui;

#[cfg(feature = "gui")]
mod gui;

fn main() {
    // 初始化 tracing 日志（默认 info 级别，可通过 RUST_LOG 环境变量覆盖）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_ansi(true)
        .init();

    #[cfg(feature = "gui")]
    {
        tracing::info!("Starting with Slint GUI");
        use tokio::sync::watch;
        use types::HeartRateReading;

        let (tx, rx_ble) = watch::channel(HeartRateReading::default());

        // Clone receivers for server and GUI
        let rx_server = rx_ble.clone();

        // Spawn tokio runtime in a background thread for BLE + HTTP
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async {
                // Spawn Axum Web server
                let server_handle = tokio::spawn(async move {
                    if let Err(err) = server::run_server(rx_server).await {
                        tracing::error!("Web 服务器错误: {err}");
                    }
                });

                // Initialize BLE adapter
                let adapter = match bluest::Adapter::default().await {
                    Some(a) => a,
                    None => {
                        tracing::error!("Bluetooth 适配器未找到（系统无蓝牙或驱动异常）");
                        tx.send_replace(HeartRateReading {
                            error: Some("蓝牙适配器未找到".into()),
                            ..Default::default()
                        });
                        let _ = server_handle.await;
                        return;
                    }
                };

                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    adapter.wait_available(),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        tracing::error!("Bluetooth 适配器不可用: {e}");
                        tx.send_replace(HeartRateReading {
                            error: Some(format!("蓝牙适配器不可用: {e}")),
                            ..Default::default()
                        });
                        let _ = server_handle.await;
                        return;
                    }
                    Err(_) => {
                        tracing::error!("Bluetooth 适配器无响应（5 秒超时），请检查蓝牙是否开启");
                        tx.send_replace(HeartRateReading {
                            error: Some("蓝牙适配器无响应，请检查蓝牙是否开启".into()),
                            ..Default::default()
                        });
                        let _ = server_handle.await;
                        return;
                    }
                }

                // Run BLE loop
                if let Err(e) = ble::run_loop(adapter, tx.clone()).await {
                    tracing::error!("蓝牙循环退出: {e}");
                    tx.send_replace(HeartRateReading {
                        error: Some(format!("蓝牙服务已停止: {e}")),
                        ..Default::default()
                    });
                }

                let _ = server_handle.await;
            });
        });

        // Run Slint GUI on the main thread (required by most windowing systems)
        if let Err(e) = gui::run(rx_ble) {
            tracing::error!("GUI 退出: {e}");
        }
    }
}
