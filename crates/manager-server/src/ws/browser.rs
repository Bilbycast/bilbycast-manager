use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::app_state::AppState;
use crate::ui::{extract_session_cookie, validate_session};

/// WebSocket handler for browser dashboard clients.
/// Requires a valid session cookie before upgrading.
pub async fn dashboard_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    // Authenticate before upgrading the WebSocket connection
    let valid = match extract_session_cookie(&headers) {
        Some(token) => validate_session(&token, &state).await,
        None => false,
    };

    if !valid {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    ws.on_upgrade(|socket| handle_dashboard_connection(socket, state))
        .into_response()
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
