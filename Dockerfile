FROM rust:1.82-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/bilbycast-manager /usr/local/bin/
COPY --from=builder /app/migrations /app/migrations
COPY --from=builder /app/config /app/config

WORKDIR /app
EXPOSE 8443

CMD ["bilbycast-manager", "serve", "--config", "/app/config/default.toml"]
