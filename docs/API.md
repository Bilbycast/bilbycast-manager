# API Reference

All endpoints except `/api/v1/auth/login` and `/health` require a valid JWT Bearer token in the `Authorization` header:

```
Authorization: Bearer <token>
```

---

## Authentication

| Method | Path                    | Description              |
|--------|-------------------------|--------------------------|
| POST   | `/api/v1/auth/login`    | Log in, returns JWT token |
| POST   | `/api/v1/auth/logout`   | Log out, invalidates session |

---

## Users

| Method | Path                    | Description              |
|--------|-------------------------|--------------------------|
| GET    | `/api/v1/users`         | List all users           |
| POST   | `/api/v1/users`         | Create a new user        |
| GET    | `/api/v1/users/{id}`    | Get user by ID           |
| PUT    | `/api/v1/users/{id}`    | Update user              |
| DELETE | `/api/v1/users/{id}`    | Delete user              |

---

## Nodes

| Method | Path                          | Description                              |
|--------|-------------------------------|------------------------------------------|
| GET    | `/api/v1/nodes`               | List all registered nodes                |
| POST   | `/api/v1/nodes`               | Register a new node (returns reg token)  |
| GET    | `/api/v1/nodes/{id}`          | Get node by ID                           |
| PUT    | `/api/v1/nodes/{id}`          | Update node metadata                     |
| DELETE | `/api/v1/nodes/{id}`          | Delete node                              |
| POST   | `/api/v1/nodes/{id}/token`    | Regenerate registration token            |
| GET    | `/api/v1/nodes/{id}/config`   | Get cached config from connected node    |
| POST   | `/api/v1/nodes/{id}/command`  | Send a command to a connected node       |

---

## Events

| Method | Path                        | Description                        |
|--------|-----------------------------|------------------------------------|
| GET    | `/api/v1/events`            | List events (supports pagination)  |
| POST   | `/api/v1/events/{id}/ack`   | Acknowledge an event               |
| GET    | `/api/v1/events/count`      | Get unacknowledged event count     |

---

## Settings

| Method | Path                  | Description              |
|--------|-----------------------|--------------------------|
| GET    | `/api/v1/settings`    | Get current settings     |
| PUT    | `/api/v1/settings`    | Update settings          |

---

## Export / Import

| Method | Path                | Description                  |
|--------|---------------------|------------------------------|
| GET    | `/api/v1/export`    | Export all data as JSON      |
| POST   | `/api/v1/import`    | Import data from JSON        |

Note: Import is currently defined but not yet fully implemented.

---

## AI

| Method | Path                           | Description                          |
|--------|--------------------------------|--------------------------------------|
| POST   | `/api/v1/ai/generate-config`   | Generate edge node config via AI     |
| POST   | `/api/v1/ai/analyze`           | AI-powered anomaly analysis          |
| POST   | `/api/v1/ai/query`             | Natural language query about nodes   |
| GET    | `/api/v1/ai/keys`              | List stored AI provider keys         |
| POST   | `/api/v1/ai/keys`              | Store an AI provider API key         |
| DELETE | `/api/v1/ai/keys`              | Delete an AI provider API key        |

---

## WebSocket Endpoints

### `/ws/dashboard`

Real-time updates for browser-based dashboards. Receives JSON messages containing aggregated node status, stats, and health data. Requires an authenticated session.

### `/ws/node`

Edge node connection endpoint. Nodes must send an `auth` message as the first WebSocket frame containing either:

- `registration_token` for first-time registration, or
- `node_id` + `node_secret` for reconnection

Message types from nodes: `stats`, `health`, `event`, `config_response`, `command_ack`, `pong`.

Message types from manager: `ping`, `command`, `register_ack`, `auth_ok`, `auth_error`.

---

## Health

| Method | Path       | Description                          |
|--------|------------|--------------------------------------|
| GET    | `/health`  | Health check (no authentication)     |
