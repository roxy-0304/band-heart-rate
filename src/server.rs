use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{extract::State, routing::get, Json, Router};

use crate::web_ui;
use tokio::sync::watch;

use crate::types::{AppState, HeartRateReading};

pub async fn run_server(rx: watch::Receiver<HeartRateReading>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/heart-rate", get(heart_rate))
        .route("/heart-rate-stream", get(heart_rate_sse))
        .route("/health", get(health))
        .with_state(AppState { rx });

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:3030").await {
        Ok(l) => {
            tracing::info!("Web UI 运行在 http://127.0.0.1:3030/");
            l
        }
        Err(e) => {
            tracing::warn!("端口 3030 绑定失败: {e}，尝试随机端口...");
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let port = l.local_addr()?.port();
            tracing::info!("Web UI 运行在 http://127.0.0.1:{port}/");
            l
        }
    };

    axum::serve(listener, app).await?;
    Ok(())
}

async fn heart_rate(State(state): State<AppState>) -> Json<HeartRateReading> {
    Json(state.rx.borrow().clone())
}

/// SSE 推送心率数据流
async fn heart_rate_sse(
    State(state): State<AppState>,
) -> Sse<impl futures_lite::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.rx.clone();
    let stream = futures_lite::stream::unfold(rx, |mut rx| async move {
        if rx.changed().await.is_ok() {
            let reading = rx.borrow().clone();
            let data = serde_json::to_string(&reading).unwrap_or_default();
            Some((Ok(Event::default().data(data)), rx))
        } else {
            None
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn index() -> axum::response::Html<&'static str> {
    axum::response::Html(web_ui::HTML)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
