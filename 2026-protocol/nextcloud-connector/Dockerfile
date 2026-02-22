# Build context is parent directory (2026-protocol) for consistency
FROM rust:1.93-slim-bookworm AS builder

# Install build dependencies for OpenSSL
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the nextcloud-connector (no local dependencies)
COPY nextcloud-connector ./nextcloud-connector

WORKDIR /app/nextcloud-connector
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/nextcloud-connector/target/release/nextcloud-connector /usr/local/bin/
EXPOSE 8083
ENV RUST_LOG=info
CMD ["nextcloud-connector", "--port", "8083", "--bind", "0.0.0.0"]
