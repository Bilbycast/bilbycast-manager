# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

bilbycast-manager is a full-stack Rust application for centralized management of distributed network devices. It combines an Axum REST/WebSocket backend, static HTML frontend with vanilla JavaScript, and SQLite database. The architecture uses a **device driver pattern** (`DeviceDriver` trait + `DriverRegistry`) to support multiple device types — currently bilbycast-edge transport nodes, with the ability to add relay servers, encoders, decoders, and third-party devices as compile-time drivers. All manager-to-device communication uses WebSocket (devices connect outbound to the manager), enabling management of devices behind firewalls/NAT.

## Build & Run Commands

```bash
# Build (debug)
cargo build

# Build (release, with LTO)
cargo build --release

# Run dev server (requires BILBYCAST_JWT_SECRET and BILBYCAST_MASTER_KEY env vars, see .env.example)
cargo run -- serve

# CLI commands
cargo run -- setup              # Initialize DB + first admin user
cargo run -- serve              # Start the server
cargo run -- reset-password --username <name>
cargo run -- export --output <file>
cargo run -- import --input <file>

# Check compilation
cargo check

# Lint
cargo clippy

# Build with optional TLS feature
cargo build --features tls
```

There are no tests in this project currently.

## Architecture

### Workspace Crates (Dependency Direction)

```
manager-ui ──→ manager-core ←── manager-server
(frontend)      (business logic)    (HTTP/WS/CLI)
```

- **manager-core** (`crates/manager-core/`) — Domain models, database operations, auth, crypto, AI providers, **device drivers**. Framework-agnostic — no Axum or web dependency. The `drivers/` module contains the `DeviceDriver` trait and `DriverRegistry`, plus implementations: `EdgeDriver` (edge transport nodes) and `RelayDriver` (relay servers).
- **manager-server** (`crates/manager-server/`) — Axum HTTP server, API handlers, auth middleware, WebSocket hubs, CLI entry point. Assembles router from public + authenticated + WS + UI routes.
- **manager-ui** (`crates/manager-ui/`) — Leptos components compiled to static HTML via `include_str!()`. Pages, layouts, and reusable card components.

### AppState (Central Shared State)

Defined in `manager-server/src/app_state.rs`. Passed to all handlers via Axum's `State` extractor:

| Field | Type | Purpose |
|-------|------|---------|
| `db` | `SqlitePool` | Database connection pool |
| `node_hub` | `Arc<NodeHub>` | WebSocket hub for all device node connections |
| `jwt_secret` | `Vec<u8>` | JWT signing key |
| `master_key` | `[u8; 32]` | AES-256-GCM master encryption key |
| `browser_stats_tx` | `broadcast::Sender<String>` | Real-time stats to browser dashboard |
| `config` | `Arc<RwLock<ServerConfig>>` | Live server configuration |
| `driver_registry` | `Arc<DriverRegistry>` | Registry of device drivers (edge, relay, etc.) |

When adding new managed device types, implement the `DeviceDriver` trait in `manager-core/src/drivers/` and register the driver at startup in `main.rs`. The hub, API, and DB handle all device types generically.

### Request Lifecycle

```
HTTP Request → CorsLayer → TraceLayer → Router
  ├── Public: /api/v1/auth/login, /api/v1/auth/logout, /health
  ├── Authenticated: /api/v1/* (auth_middleware layer)
  │   └── JWT from Authorization header or session cookie
  │   └── Validates user active + not expired
  │   └── Injects AuthUser {user_id, role, session_id, allowed_node_ids}
  ├── WebSocket: /ws/node, /ws/dashboard
  └── UI: static HTML pages
```

### Authentication & Authorization

- **Password hashing:** Argon2id (`manager-core/src/auth/password.rs`), with timing-safe login (dummy hash on unknown users prevents username enumeration)
- **Sessions:** JWT with HMAC-SHA256, delivered as `httpOnly; Secure; SameSite=Strict` cookie. Claims include `sub` (user_id), `role`, `jti` (session_id) (`auth/jwt.rs`)
- **Session revocation:** Logout adds `jti` to `revoked_sessions` SQLite table; middleware rejects revoked tokens (`db/sessions.rs`)
- **Login rate limiting:** 5 attempts per 60s per IP, returns HTTP 429 when exceeded (`middleware/rate_limit.rs`)
- **RBAC:** 4-level hierarchy — Viewer(0) < Operator(1) < Admin(2) < SuperAdmin(3). Checked via `UserRole::has_permission(minimum_role)` (`auth/rbac.rs`)
- **Node-level access:** `AuthUser.allowed_node_ids` — `None` means all nodes, `Some(vec)` restricts to listed node IDs
- **Middleware:** `manager-server/src/middleware/auth.rs` — extracts JWT from cookie (primary) or Authorization header (fallback), validates, checks revocation, enforces CSRF on mutating requests
- **CSRF:** Double-submit cookie pattern. Login sets non-httpOnly `csrf_token` cookie; middleware validates `X-CSRF-Token` header matches cookie on POST/PUT/PATCH/DELETE. Constant-time comparison (`auth/csrf.rs`)
- **CORS:** Restricted to same-origin only; cross-origin API requests are blocked
- **TLS:** Mandatory — server requires `BILBYCAST_TLS_CERT` and `BILBYCAST_TLS_KEY` to start. Edge and relay clients enforce `wss://` URLs. Self-signed certs are detected at startup; all UI pages show a warning banner when using self-signed certs. Certs can be uploaded via `POST /api/v1/settings/tls/upload` or the Settings page
- **Self-signed cert acceptance:** Edge/relay clients support `accept_self_signed_cert: true` in their manager config for dev/testing (disables cert validation)

### WebSocket Architecture

**Node Hub** (`manager-server/src/ws/node_hub.rs`) — the most complex component:

1. **Connection auth** (10s timeout): Node sends first message with either `registration_token` (new node) or `node_id + node_secret` (reconnection)
2. **Registration flow:** Token lookup → generate secret → encrypt with master_key → store in DB → return `register_ack` with credentials
3. **Reconnection flow:** Decrypt stored secret → compare → return `auth_ok`
4. **Main loop:** `tokio::select!` over socket recv (stats/health/events from node) and mpsc recv (commands to node)
5. **State:** Each connected node tracked as `ConnectedNode` in `DashMap<String, ConnectedNode>` with `device_type`, cached config, stats, health
6. **Anti-bruteforce:** `NodeAuthLimiter` — 5 failures per 60s window per node_id
7. **Driver-aware broadcast:** Dashboard updates include `device_type` and `driver_metrics` extracted by the node's registered driver

**Communication:** All manager→node communication uses WebSocket commands (nodes connect outbound to manager). No direct HTTP calls to nodes — this enables management of devices behind firewalls/NAT.

**Message protocol** (`manager-core/src/models/ws_protocol.rs`): JSON envelope `{"msg_type": "...", "payload": {...}}`

- Node → Manager: `stats`, `health`, `event`, `config_response`, `command_ack`, `pong`
- Manager → Node: `command` with action payload (GetConfig, UpdateConfig, CreateFlow, DeleteFlow, StartFlow, StopFlow, etc.)

**Browser Dashboard** (`ws/browser.rs`) — One-way broadcast of aggregated node stats to all connected browsers via `broadcast::channel(256)`. Currently has no authentication.

### Encryption at Rest

`manager-core/src/crypto.rs` — Single file, used for all secret storage:

- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key derivation:** HKDF-SHA256 from `BILBYCAST_MASTER_KEY` with salt `"bilbycast-manager-master-key-v1"`
- **Storage format:** Base64(12-byte-nonce || ciphertext)
- **Used for:** Node auth secrets (`auth_client_secret_enc`), AI API keys (`api_key_enc`), tunnel PSKs (`tunnel_psk_enc`)

### AI Integration

Trait-based provider abstraction in `manager-core/src/ai/`:

- **`AiProviderTrait`** (async_trait): `generate_flow_config()`, `analyze_anomaly()`, `answer_query()`
- **Implementations:** OpenAI (`openai.rs`), Anthropic (`anthropic.rs`), Gemini (`gemini.rs`)
- **Context building:** `config_gen.rs` assembles protocol docs + flow config JSON schema + node info
- **API handlers:** `manager-server/src/api/ai.rs` — keys are encrypted/decrypted per-request using master_key

### Database

SQLite via SQLx with compile-time query checking. Migrations in `/migrations/`. Key design choices:

- **JSON blobs** for flexible fields: `last_health`, `metadata`, `details`, `allowed_node_ids`, `associated_flow_ids`
- **Encrypted fields** suffixed `_enc`: `auth_client_secret_enc`, `api_key_enc`, `tunnel_psk_enc`
- **Row mapping pattern:** Internal `*Row` structs (sqlx::FromRow) map to domain model structs with type conversions (RFC3339 strings → chrono::DateTime, JSON strings → serde_json::Value)
- **Fire-and-forget audit:** All mutations call `db::audit::log_audit()` but errors are swallowed (`let _ = ...`)

### API Handler Patterns

All handlers in `manager-server/src/api/`. Common patterns:

```rust
// 1. Authorization check
if !auth.role.has_permission(UserRole::Admin) {
    return Err(StatusCode::FORBIDDEN);
}
// 2. Node access check (for node-scoped operations)
if !auth.can_access_node(&node_id) {
    return Err(StatusCode::FORBIDDEN);
}
// 3. DB operation
let result = manager_core::db::module::operation(&state.db, &req)
    .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
// 4. Audit log (fire-and-forget)
let _ = manager_core::db::audit::log_audit(&state.db, ...).await;
```

### Frontend

Static HTML pages embedded via `include_str!()` in `manager-server/src/ui/`. Leptos components in `manager-ui/src/`:

- **Layouts:** `AuthLayout` (login), `MainLayout` (sidebar + header + content area)
- **Pages:** dashboard, topology, node_detail, node_config, events, users, settings, ai_assistant, ai_settings
- **Components:** `NodeCard`, `FlowCard`, common components (modal, toast, badge — placeholder)
- **Styling:** Tailwind CSS dark theme (slate palette), configured in `tailwind.config.js`

## Environment Variables

Required:
- `BILBYCAST_JWT_SECRET` — 64-char hex string (32 bytes), validated on startup (rejects weak/short values)
- `BILBYCAST_MASTER_KEY` — 64-char hex string (32 bytes), validated on startup

Required:
- `BILBYCAST_TLS_CERT` / `BILBYCAST_TLS_KEY` — TLS certificate and key paths (server will not start without TLS)

Optional:
- `BILBYCAST_PORT` — Override listen port (default: 8443)
- `BILBYCAST_DATABASE_URL` — Override SQLite path (default: `sqlite:bilbycast-manager.db?mode=rwc`)

See `.env.example` for a template.

## Extensibility Guide — Adding New Device Types (Driver Pattern)

The architecture uses a **device driver pattern** for managing different types of network devices. All device types share the same hub, DB schema, API routes, and WebSocket protocol. Device-specific behavior is encapsulated in drivers.

### Currently registered drivers:
- **EdgeDriver** (`edge.rs`) — bilbycast-edge transport nodes. Commands: get_config, update_config, create/update/delete/start/stop/restart_flow, add/remove_output
- **RelayDriver** (`relay.rs`) — bilbycast-relay servers. Commands: get_config, disconnect_edge, close_tunnel, list_tunnels, list_edges

### To add a new device type (e.g., encoder, decoder):

1. **Driver** (`manager-core/src/drivers/new_device.rs`): Implement the `DeviceDriver` trait:
   - `device_type()` / `display_name()` — identifiers
   - `extract_metrics()` — parse device stats for dashboard display
   - `supported_commands()` / `validate_command()` — device-specific commands
   - `ai_context()` — protocol docs for AI assistant
2. **Register** in `manager-server/src/main.rs`: `registry.register(Arc::new(NewDeviceDriver::new()));`
3. **Create nodes** with `device_type: "new_device"` via the existing `POST /api/v1/nodes` API
4. **UI** (`manager-server/src/ui/`): Add device-specific page if needed. The existing node config page works for any device type.

That's it. The hub, DB, auth, API routes, WebSocket protocol, events, export, and audit logging all work automatically for any registered device type. The `nodes` table has a `device_type` column, and `GET /api/v1/nodes?device_type=relay` supports filtering.

### Key files:
- `manager-core/src/drivers/mod.rs` — `DeviceDriver` trait, `DriverRegistry`, shared types
- `manager-core/src/drivers/edge.rs` — Edge transport node driver
- `manager-core/src/drivers/relay.rs` — Relay server driver
- `GET /api/v1/device-types` — Lists all registered drivers with capabilities

### UI device-type awareness:
The dashboard, topology, node detail, and node config pages all read `device_type` from the WebSocket broadcast and render device-specific views. Relay nodes show purple accent styling, tunnel-focused displays, and hide edge-specific sections (flows, AI config generation).
