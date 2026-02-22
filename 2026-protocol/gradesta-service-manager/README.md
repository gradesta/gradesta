# Gradesta Service Manager

Manages local Gradesta services using Docker Compose and Caddy as a reverse proxy.

## Usage

```bash
cd gradesta-service-manager
nix-shell

# Install a service
cargo run -- install-service /path/to/service

# Start all services
cargo run -- start

# Stop all services
cargo run -- stop

# List services and status
cargo run -- list

# Redeploy a service after code changes
cargo run -- redeploy <service-name>

# Remove a service
cargo run -- remove <service-name>
```

## Configuration

Services are registered in `~/.config/gradesta/services/registry.toml`.

Generated files:
- `~/.config/gradesta/services/docker-compose.yml` - Docker Compose configuration
- `~/.config/gradesta/services/Caddyfile` - Caddy reverse proxy configuration
- `~/.config/gradesta/services/data/<service-name>/` - Persistent data for each service

## Persistent Storage

Each service has a volume mounted at `/data` inside the container. This maps to:

```
~/.config/gradesta/services/data/<service-name>/
```

Services should store any persistent state (credentials, caches, databases) in `/data` to preserve it across container restarts.

### Example usage in a service

```rust
// Get the data directory path
let data_dir = std::path::Path::new("/data");

// Store credentials
let creds_path = data_dir.join("credentials.json");
std::fs::write(&creds_path, credentials_json)?;

// Load on startup
if creds_path.exists() {
    let credentials = std::fs::read_to_string(&creds_path)?;
}
```

## Network Configuration

All services are on the `gradesta-local` Docker network with the alias `gradesta-local-services`. This allows:

- Services to communicate with each other using service names (e.g., `nextcloud-connector:8083`)
- The browser to translate `localhost` URLs to `gradesta-local-services` for elf connectivity

Caddy exposes port 19333 on the host and proxies requests to services:
- `http://localhost:19333/<service-name>/` → `<service-name>:<internal-port>`

## Development Workflow

Each service has its own `shell.nix` for local development. To rebuild and redeploy a service after code changes:

```bash
# From the service-manager directory
cd gradesta-service-manager
nix-shell --run "cargo run -- redeploy <service-name>"

# Examples:
nix-shell --run "cargo run -- redeploy nextcloud-connector"
nix-shell --run "cargo run -- redeploy pig-latin-elf"
```

This will:
1. Rebuild the Docker image from the service's source directory
2. Recreate the container with the new image
3. Restart the service

Note: Services are built inside Docker containers, not on the host. The Dockerfile in each service's directory controls the build process.
