# Security Documentation

## Architecture Overview

bilbycast-manager handles several categories of sensitive data:

- **User passwords** -- hashed with Argon2id, never stored in plaintext
- **JWT session tokens** -- signed with HMAC-SHA256 using `BILBYCAST_JWT_SECRET`
- **Node secrets** -- encrypted at rest with AES-256-GCM using a key derived from `BILBYCAST_MASTER_KEY`
- **AI API keys** -- encrypted at rest with AES-256-GCM (same derived key)
- **Configuration** -- non-secret settings stored in plaintext TOML

All cryptographic secrets are loaded from environment variables at startup. The server refuses to start if secrets are missing, empty, too short (< 16 characters), or contain known weak/default values.

---

## Secrets Management

### Environment Variables

Two secrets are **required** and must be set before starting the server:

| Variable              | Purpose                                           |
|-----------------------|---------------------------------------------------|
| `BILBYCAST_JWT_SECRET`| HMAC key for signing/verifying JWT session tokens  |
| `BILBYCAST_MASTER_KEY`| Passphrase for deriving the AES-256-GCM encryption key |

Generate them with:

```bash
openssl rand -hex 32
```

These must **never** appear in `config/default.toml` or be committed to version control.

### Key Derivation

The `BILBYCAST_MASTER_KEY` passphrase is run through HKDF-SHA256 to produce a 32-byte AES key:

1. **Extract**: `HMAC-SHA256(salt, passphrase)` where salt = `bilbycast-manager-master-key-v1`
2. **Expand**: `HMAC-SHA256(PRK, "aes-256-gcm-encryption" || 0x01)`

This produces a uniformly distributed 256-bit key suitable for AES-256-GCM.

### Stored Secrets Encryption

Node secrets and AI API keys are encrypted before being written to the SQLite database:

- **Algorithm**: AES-256-GCM (authenticated encryption)
- **Nonce**: 12 bytes, randomly generated per encryption operation
- **Storage format**: Base64-encoded concatenation of `nonce (12 bytes) || ciphertext`

### .env File Permissions

The `.env` file contains the two master secrets and should be restricted:

```bash
chmod 600 .env
```

Ensure it is listed in `.gitignore`.

---

## User Authentication

### Password Hashing

User passwords are hashed with **Argon2id** (the default parameters from the `argon2` crate). Plaintext passwords are never stored or logged.

Password requirements:
- Minimum 8 characters, maximum 128 characters
- Must contain at least one uppercase letter, one lowercase letter, and one digit

### JWT Session Tokens

After successful login, the server issues a JWT containing:

| Claim | Content                          |
|-------|----------------------------------|
| `sub` | User ID                          |
| `role`| User role (e.g., `super_admin`)  |
| `jti` | Session ID (for revocation)      |
| `iat` | Issued-at timestamp              |
| `exp` | Expiration timestamp             |
| `iss` | `bilbycast-manager`              |

Tokens are signed with HMAC-SHA256 using `BILBYCAST_JWT_SECRET`. The issuer is validated on decode.

### Role-Based Access Control (RBAC)

Four roles are defined, in ascending privilege order:

| Role          | Level | Typical Permissions                                |
|---------------|-------|----------------------------------------------------|
| `viewer`      | 0     | Read-only access to dashboards and node status      |
| `operator`    | 1     | Start/stop flows, acknowledge events                |
| `admin`       | 2     | Create/delete nodes and users, manage settings      |
| `super_admin` | 3     | Full access including managing other admins          |

Permission checks enforce that the user's role level is >= the required level for the operation.

### Temporary Users

Users can be marked as temporary with an `expires_at` timestamp. Expired accounts are denied access at permission check time.

### CSRF Protection

CSRF tokens are generated as 32-character hex strings (128 bits of randomness). Verification uses constant-time comparison to prevent timing attacks.

---

## Node Authentication

### Two-Stage Registration

Edge nodes authenticate with the manager using a two-stage process:

1. **Registration**: The manager administrator creates a node entry via the API, which generates a one-time registration token. The edge node connects to `/ws/node` and sends an `auth` message containing `registration_token`. On success, the manager responds with `register_ack` containing a permanent `node_id` and `node_secret`. The registration token is consumed and cannot be reused.

2. **Reconnection**: On subsequent connections, the edge node sends `node_id` and `node_secret` in the `auth` message. The manager decrypts the stored node secret and compares.

### Credential Transport

Node credentials are sent via the first WebSocket text frame after the connection is established -- **not** in URL query parameters. This prevents secrets from appearing in server access logs, proxy logs, or browser history.

### Rate Limiting

Failed authentication attempts are tracked per identifier (node_id or token prefix):

- **Threshold**: 5 failed attempts within a 60-second window
- **Lockout**: The identifier is locked out for the remainder of the 60-second window
- **Recovery**: Successful authentication clears the failure counter
- **Cleanup**: Expired tracking entries are periodically removed

### Node Secrets at Rest

Node secrets are encrypted with AES-256-GCM before storage in the database. The encryption key is derived from `BILBYCAST_MASTER_KEY` via HKDF-SHA256 (see above).

---

## Transport Security

### TLS (HTTPS/WSS)

TLS is optional and requires building with the `tls` feature:

```bash
cargo build --release --features tls
```

Configure via environment variables or `config/default.toml`:

```bash
BILBYCAST_TLS_CERT=/path/to/cert.pem
BILBYCAST_TLS_KEY=/path/to/key.pem
```

Or in TOML:

```toml
[tls]
cert_path = "certs/server.crt"
key_path = "certs/server.key"
```

TLS is provided by **rustls** (a pure-Rust TLS implementation).

### Without TLS

When TLS is not configured, the server runs in plaintext HTTP/WS mode. A warning is logged at startup:

> TLS not configured -- running in plaintext HTTP/WS mode.

This is acceptable for development but **not recommended for production**, especially when edge nodes transmit credentials over the network.

### Edge Node Connections

In production, edge nodes should connect using `wss://` URLs to ensure credentials are encrypted in transit.

---

## API Security

### Authentication Requirements

All API endpoints require a valid JWT Bearer token in the `Authorization` header, with two exceptions:

- `POST /api/v1/auth/login` -- used to obtain a token
- `GET /health` -- unauthenticated health check

### CORS

CORS is configured via `tower_http::cors::CorsLayer`. The current configuration is permissive; for production, restrict allowed origins.

---

## AI API Key Storage

AI provider API keys (OpenAI, Anthropic, etc.) are:

- Encrypted with AES-256-GCM before storage in the database
- Displayed as masked values (asterisks) in the UI
- Decrypted only when needed to make API calls to the provider

---

## What Is NOT Yet Implemented

The following security features are not currently present:

- **Mutual TLS (mTLS)** for node authentication -- nodes authenticate via WebSocket message, not client certificates
- **Hardware Security Module (HSM) support** -- master keys are stored in environment variables or `.env` files
- **Audit log signing** -- events are logged to the database but not cryptographically signed
- **IP allowlisting** for node connections -- any IP can attempt to connect to `/ws/node`
- **Account lockout for user login** -- rate limiting currently only applies to node authentication, not user login
- **Import functionality** -- the `import` CLI command is defined but not yet implemented

---

## Recommendations for Production Deployment

1. **Enable TLS** -- build with `--features tls` and provide valid certificates. Use `wss://` URLs for all edge node connections.

2. **Restrict `.env` permissions** -- `chmod 600 .env` and ensure it is owned by the service user.

3. **Use a reverse proxy** -- place the server behind nginx or similar for additional protection (rate limiting, request size limits, IP filtering).

4. **Restrict CORS origins** -- update the CORS configuration to allow only your specific frontend domain(s).

5. **Rotate secrets periodically** -- generate new `BILBYCAST_JWT_SECRET` and `BILBYCAST_MASTER_KEY` values. Rotating `JWT_SECRET` invalidates all active sessions. Rotating `MASTER_KEY` requires re-encrypting stored node secrets and API keys.

6. **Back up the database** -- the SQLite database contains encrypted secrets, user accounts, and event history.

7. **Monitor logs** -- watch for repeated authentication failures, which may indicate brute-force attempts.

8. **Run as a non-root user** -- create a dedicated service account with minimal filesystem permissions.
