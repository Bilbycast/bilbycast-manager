-- Store tunnel bind secret encrypted at rest so tunnels can be re-pushed on reconnection
ALTER TABLE tunnels ADD COLUMN tunnel_bind_secret_enc TEXT;
