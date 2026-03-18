pub mod types;

pub use types::*;

// The node_client module defines the WebSocket message types used between
// the manager and edge nodes. All communication is WebSocket-only.
// See models/ws_protocol.rs for the actual message types.
//
// The manager-server crate's ws/node_hub.rs handles the actual WebSocket
// connections and message routing.
