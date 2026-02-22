# Build context is parent directory (2026-protocol) to access local dependencies
FROM rust:1.93-slim-bookworm AS builder
WORKDIR /app

# Copy the service and its local dependencies
COPY pig-latin-elf ./pig-latin-elf
COPY gradesta-elf ./gradesta-elf

WORKDIR /app/pig-latin-elf
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/pig-latin-elf/target/release/pig-latin-elf /usr/local/bin/
EXPOSE 9001
CMD ["pig-latin-elf", "--port", "9001"]
