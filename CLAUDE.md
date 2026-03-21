# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

bilbycast-manager is a full-stack Rust application for centralized management of distributed media transport (broadcast) edge nodes. It combines an Axum REST/WebSocket backend, Leptos frontend (served as static HTML), and SQLite database. The architecture is designed to be extensible to manage additional network module types beyond edge nodes.

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

- **manager-core** (`crates/manager-core/`) — Domain models, database operations, auth, crypto, AI providers. Framework-agnostic — no Axum or web dependency. This is the extension point for new module types.
- **manager-server** (`crates/manager-server/`) — Axum HTTP server, API handlers, auth middleware, WebSocket hubs, CLI entry point. Assembles router from public + authenticated + WS + UI routes.
- **manager-ui** (`crates/manager-ui/`) — Leptos components compiled to static HTML via `include_str!()`. Pages, layouts, and reusable card components.

### AppState (Central Shared State)

Defined in `manager-server/src/app_state.rs`. Passed to all handlers via Axum's `State` extractor:

| Field | Type | Purpose |
|-------|------|---------|
| `db` | `SqlitePool` | Database connection pool |
| `node_hub` | `Arc<NodeHub>` | WebSocket hub for edge node connections |
| `jwt_secret` | `Vec<u8>` | JWT signing key |
| `master_key` | `[u8; 32]` | AES-256-GCM master encryption key |
| `browser_stats_tx` | `broadcast::Sender<String>` | Real-time stats to browser dashboard |
| `config` | `Arc<RwLock<ServerConfig>>` | Live server configuration |

When adding new managed module types (e.g., relay nodes, encoders), add their hub to AppState following the NodeHub pattern.

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

- **Password hashing:** Argon2id (`manager-core/src/auth/password.rs`)
- **Sessions:** JWT with HMAC-SHA256, claims include `sub` (user_id), `role`, `jti` (session_id) (`auth/jwt.rs`)
- **RBAC:** 4-level hierarchy — Viewer(0) < Operator(1) < Admin(2) < SuperAdmin(3). Checked via `UserRole::has_permission(minimum_role)` (`auth/rbac.rs`)
- **Node-level access:** `AuthUser.allowed_node_ids` — `None` means all nodes, `Some(vec)` restricts to listed node IDs
- **Middleware:** `manager-server/src/middleware/auth.rs` — extracts and validates JWT, loads user from DB
- **CSRF:** Random 128-bit tokens with constant-time comparison (`auth/csrf.rs`)

### WebSocket Architecture

**Node Hub** (`manager-server/src/ws/node_hub.rs`) — the most complex component:

1. **Connection auth** (10s timeout): Node sends first message with either `registration_token` (new node) or `node_id + node_secret` (reconnection)
2. **Registration flow:** Token lookup → generate secret → encrypt with master_key → store in DB → return `register_ack` with credentials
3. **Reconnection flow:** Decrypt stored secret → compare → return `auth_ok`
4. **Main loop:** `tokio::select!` over socket recv (stats/health/events from node) and mpsc recv (commands to node)
5. **State:** Each connected node tracked as `ConnectedNode` in `DashMap<String, ConnectedNode>` with cached config, stats, health
6. **Anti-bruteforce:** `NodeAuthLimiter` — 5 failures per 60s window per node_id

**Message protocol** (`manager-core/src/models/ws_protocol.rs`): JSON envelope `{"msg_type": "...", "payload": {...}}`

- Node → Manager: `stats`, `health`, `event`, `config_response`, `command_ack`, `pong`
- Manager → Node: `command` with `CommandAction` enum (GetConfig, UpdateConfig, CreateFlow, DeleteFlow, StartFlow, StopFlow, etc.)

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

Optional:
- `BILBYCAST_PORT` — Override listen port (default: 8443)
- `BILBYCAST_DATABASE_URL` — Override SQLite path (default: `sqlite:bilbycast-manager.db?mode=rwc`)
- `BILBYCAST_TLS_CERT` / `BILBYCAST_TLS_KEY` — TLS certificate and key paths (requires `tls` feature)

See `.env.example` for a template.

## Extensibility Guide — Adding New Network Module Types

The current architecture manages "nodes" (edge transport devices). To add a new module type (e.g., relay servers, encoders, decoders, monitoring probes):

1. **Models** (`manager-core/src/models/`): Add `new_module.rs` with status enum, domain struct, create/update DTOs. Follow the `node.rs` pattern.
2. **Database** (`manager-core/src/db/`): Add `new_module.rs` with CRUD operations + `*Row` mapping structs. Add a migration in `/migrations/`.
3. **WebSocket hub** (if real-time): Add `new_module_hub.rs` in `manager-server/src/ws/` following `node_hub.rs` pattern. Add the hub to `AppState`.
4. **API handlers** (`manager-server/src/api/`): Add `new_module.rs` with CRUD + command endpoints. Register routes in `api/mod.rs` under the authenticated router.
5. **WS protocol** (`models/ws_protocol.rs`): Add message types for the new module if it communicates via WebSocket.
6. **UI** (`manager-ui/src/`): Add page(s) and card component. Register route in `app.rs`.
7. **Export** (`manager-core/src/export.rs`): Add to `ExportData` struct and `export_all()` function.

Each module type should be self-contained within its own files across crates, communicating through AppState and the database.
