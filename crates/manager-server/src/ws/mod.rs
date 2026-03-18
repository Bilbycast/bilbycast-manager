pub mod node_hub;
pub mod browser;

use axum::Router;
use axum::routing::get;

use crate::app_state::AppState;

pub fn build_ws_router(state: AppState) -> Router<AppState> {
    let _ = state;
    Router::new()
        .route("/ws/node", get(node_hub::node_ws_handler))
        .route("/ws/dashboard", get(browser::dashboard_ws_handler))
}
