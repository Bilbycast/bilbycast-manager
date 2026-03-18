use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};

use crate::ws::node_hub::NodeHub;
use crate::ServerConfig;

/// Shared application state passed to all request handlers.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool.
    pub db: SqlitePool,
    /// WebSocket hub managing all edge node connections.
    pub node_hub: Arc<NodeHub>,
    /// JWT signing secret.
    pub jwt_secret: Vec<u8>,
    /// Master encryption key for stored secrets (API keys, node credentials).
    pub master_key: [u8; 32],
    /// Broadcast channel for pushing aggregated stats to browser WebSocket clients.
    pub browser_stats_tx: broadcast::Sender<String>,
    /// Server configuration.
    pub config: Arc<RwLock<ServerConfig>>,
}
