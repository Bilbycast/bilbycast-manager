use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::app_state::AppState;

/// WebSocket handler for browser dashboard clients.
pub async fn dashboard_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_dashboard_connection(socket, state))
}

async fn handle_dashboard_connection(mut socket: WebSocket, state: AppState) {
    let mut rx = state.browser_stats_tx.subscribe();

    tracing::info!("Browser dashboard client connected");

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Browser client lagged, skipped {n} messages");
                    }
                    Err(_) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("Browser dashboard client disconnected");
}
