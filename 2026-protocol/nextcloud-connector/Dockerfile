# Build context is parent directory (2026-protocol) for consistency
FROM rust:1.93-slim-bookworm AS builder

# Install build dependencies for OpenSSL
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app/nextcloud-connector

# Copy only manifests first for dependency caching
COPY nextcloud-connector/Cargo.toml nextcloud-connector/Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer is cached if Cargo.toml/Cargo.lock don't change)
RUN cargo build --release

# Remove the dummy build artifacts for our crate (keep deps)
RUN rm -rf src target/release/deps/nextcloud_connector* target/release/nextcloud-connector*

# Copy actual source code
COPY nextcloud-connector/src ./src

# Build the real application (deps are cached, only our code rebuilds)
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/nextcloud-connector/target/release/nextcloud-connector /usr/local/bin/
EXPOSE 8083
ENV RUST_LOG=info
CMD ["nextcloud-connector", "--port", "8083", "--bind", "0.0.0.0"]
