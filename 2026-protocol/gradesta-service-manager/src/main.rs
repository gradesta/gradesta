mod build;
mod caddy;
mod config;
mod docker;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gradesta-service-manager")]
#[command(about = "Manage local Gradesta services using Docker Compose + Caddy")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a service from a local source directory
    InstallService {
        /// Path to the service source directory
        #[arg(long = "local-dev")]
        local_dev: PathBuf,
    },

    /// Rebuild and redeploy a service after code changes
    Redeploy {
        /// Service name to redeploy (omit for all)
        service: Option<String>,

        /// Redeploy all services
        #[arg(long)]
        all: bool,

        /// Skip --no-cache for faster rebuilds
        #[arg(long)]
        quick: bool,
    },

    /// Start all services
    Start,

    /// Stop all services
    Stop,

    /// List services with status
    List,

    /// Remove a service
    Remove {
        /// Name of the service to remove
        service: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::InstallService { local_dev } => {
            install_service(&local_dev).await?;
        }
        Commands::Redeploy {
            service,
            all,
            quick,
        } => {
            redeploy(service, all, quick).await?;
        }
        Commands::Start => {
            start_services().await?;
        }
        Commands::Stop => {
            stop_services().await?;
        }
        Commands::List => {
            list_services().await?;
        }
        Commands::Remove { service } => {
            remove_service(&service).await?;
        }
    }

    Ok(())
}

async fn install_service(source_dir: &PathBuf) -> Result<()> {
    let source_dir = source_dir.canonicalize()?;
    println!("Installing service from: {}", source_dir.display());

    // Detect service name from Cargo.toml
    let cargo_toml_path = source_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        anyhow::bail!("No Cargo.toml found in {}", source_dir.display());
    }

    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)?;
    let cargo: toml::Value = toml::from_str(&cargo_toml)?;

    let name = cargo
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Could not find package name in Cargo.toml"))?
        .to_string();

    // Detect service type (elf vs server) by checking for gradesta-elf dependency
    let is_elf = cargo
        .get("dependencies")
        .and_then(|d| d.get("gradesta-elf"))
        .is_some();

    let service_type = if is_elf { "elf" } else { "server" };

    // Determine internal port based on type
    let internal_port = if is_elf { 9001 } else { 8083 };

    println!("  Name: {}", name);
    println!("  Type: {}", service_type);
    println!("  Port: {}", internal_port);

    // Load or create registry
    let mut registry = config::ServiceRegistry::load()?;

    // Check if service already exists
    if registry.services.iter().any(|s| s.name == name) {
        anyhow::bail!("Service '{}' already exists. Use 'remove' first.", name);
    }

    // Generate Dockerfile if it doesn't exist
    let dockerfile_path = source_dir.join("Dockerfile");
    if !dockerfile_path.exists() {
        println!("  Generating Dockerfile...");
        let dockerfile_content = build::generate_dockerfile(&name, internal_port, is_elf)?;
        std::fs::write(&dockerfile_path, dockerfile_content)?;
    } else {
        println!("  Using existing Dockerfile");
    }

    // Add service to registry
    let service = config::Service {
        name: name.clone(),
        service_type: service_type.to_string(),
        source_dir: source_dir.to_string_lossy().to_string(),
        internal_port,
        enabled: true,
    };
    registry.services.push(service);
    registry.save()?;

    // Regenerate docker-compose.yml and Caddyfile
    docker::generate_compose(&registry)?;
    caddy::generate_caddyfile(&registry)?;
    config::generate_browser_services(&registry)?;

    println!("Service '{}' installed successfully!", name);
    println!("Run 'gradesta-service-manager start' to start all services.");

    Ok(())
}

async fn redeploy(service: Option<String>, all: bool, quick: bool) -> Result<()> {
    let registry = config::ServiceRegistry::load()?;
    let services_dir = config::services_dir()?;

    let services_to_redeploy: Vec<&config::Service> = if all {
        registry.services.iter().filter(|s| s.enabled).collect()
    } else if let Some(ref name) = service {
        let svc = registry
            .services
            .iter()
            .find(|s| s.name == *name)
            .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", name))?;
        vec![svc]
    } else {
        anyhow::bail!("Specify a service name or use --all");
    };

    for svc in services_to_redeploy {
        println!("Redeploying {}...", svc.name);

        // Build
        let mut build_args = vec!["compose", "build"];
        if !quick {
            build_args.push("--no-cache");
        }
        build_args.push(&svc.name);

        let status = tokio::process::Command::new("docker")
            .args(&build_args)
            .current_dir(&services_dir)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Build failed for {}", svc.name);
        }

        // Recreate container
        let status = tokio::process::Command::new("docker")
            .args(["compose", "up", "-d", "--force-recreate", &svc.name])
            .current_dir(&services_dir)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Failed to recreate container for {}", svc.name);
        }

        println!("  {} redeployed successfully", svc.name);
    }

    Ok(())
}

async fn start_services() -> Result<()> {
    let registry = config::ServiceRegistry::load()?;
    let services_dir = config::services_dir()?;

    // Regenerate files in case of manual edits to registry.toml
    docker::generate_compose(&registry)?;
    caddy::generate_caddyfile(&registry)?;
    config::generate_browser_services(&registry)?;

    println!("Starting services...");

    let status = tokio::process::Command::new("docker")
        .args(["compose", "up", "-d", "--build"])
        .current_dir(&services_dir)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to start services");
    }

    println!("Services started!");
    println!(
        "Caddy proxy available at: http://localhost:{}",
        registry.caddy_port
    );

    // List service URLs
    for service in &registry.services {
        if service.enabled {
            println!(
                "  - {}: http://localhost:{}/{}",
                service.name, registry.caddy_port, service.name
            );
        }
    }

    Ok(())
}

async fn stop_services() -> Result<()> {
    let services_dir = config::services_dir()?;

    println!("Stopping services...");

    let status = tokio::process::Command::new("docker")
        .args(["compose", "down"])
        .current_dir(&services_dir)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to stop services");
    }

    println!("Services stopped.");

    Ok(())
}

async fn list_services() -> Result<()> {
    let registry = config::ServiceRegistry::load()?;
    let services_dir = config::services_dir()?;

    if registry.services.is_empty() {
        println!("No services installed.");
        return Ok(());
    }

    // Get container status
    let output = tokio::process::Command::new("docker")
        .args(["compose", "ps", "--format", "json"])
        .current_dir(&services_dir)
        .output()
        .await?;

    let running_services: std::collections::HashSet<String> = if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .filter_map(|line| {
                let v: serde_json::Value = serde_json::from_str(line).ok()?;
                let name = v.get("Service")?.as_str()?;
                let state = v.get("State")?.as_str()?;
                if state == "running" {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    println!(
        "{:<25} {:<10} {:<10} {:<8}",
        "NAME", "TYPE", "PORT", "STATUS"
    );
    println!("{}", "-".repeat(55));

    for service in &registry.services {
        let status = if !service.enabled {
            "disabled"
        } else if running_services.contains(&service.name) {
            "running"
        } else {
            "stopped"
        };

        println!(
            "{:<25} {:<10} {:<10} {:<8}",
            service.name, service.service_type, service.internal_port, status
        );
    }

    Ok(())
}

async fn remove_service(name: &str) -> Result<()> {
    let mut registry = config::ServiceRegistry::load()?;

    let idx = registry
        .services
        .iter()
        .position(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("Service '{}' not found", name))?;

    registry.services.remove(idx);
    registry.save()?;

    // Regenerate files
    docker::generate_compose(&registry)?;
    caddy::generate_caddyfile(&registry)?;
    config::generate_browser_services(&registry)?;

    println!("Service '{}' removed.", name);
    println!("Run 'gradesta-service-manager start' to apply changes.");

    Ok(())
}
