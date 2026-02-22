use anyhow::Result;

/// Generate a Dockerfile for a service.
/// Note: The docker-compose.yml uses the parent directory as build context,
/// so paths in the Dockerfile are relative to the parent directory.
pub fn generate_dockerfile(binary_name: &str, port: u16, has_gradesta_elf_dep: bool) -> Result<String> {
    let copy_deps = if has_gradesta_elf_dep {
        format!("COPY {} ./{}\nCOPY gradesta-elf ./gradesta-elf", binary_name, binary_name)
    } else {
        format!("COPY {} ./{}", binary_name, binary_name)
    };

    let dockerfile = format!(
        r#"# Build context is parent directory (2026-protocol) to access local dependencies
FROM rust:1.84-slim-bookworm AS builder
WORKDIR /app

# Copy the service and its local dependencies
{copy_deps}

WORKDIR /app/{binary_name}
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/{binary_name}/target/release/{binary_name} /usr/local/bin/
EXPOSE {port}
CMD ["{binary_name}", "--port", "{port}", "--bind", "0.0.0.0"]
"#,
        copy_deps = copy_deps,
        binary_name = binary_name,
        port = port
    );

    Ok(dockerfile)
}
