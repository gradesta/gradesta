use crate::config::{services_dir, ServiceRegistry};
use anyhow::Result;

pub fn generate_compose(registry: &ServiceRegistry) -> Result<()> {
    let services_dir = services_dir()?;
    let compose_path = services_dir.join("docker-compose.yml");

    let mut content = String::new();

    content.push_str("version: \"3.8\"\n\n");

    content.push_str("networks:\n");
    content.push_str("  gradesta-local:\n");
    content.push_str("    driver: bridge\n\n");

    content.push_str("services:\n");

    // Caddy service
    content.push_str("  caddy:\n");
    content.push_str("    image: caddy:2-alpine\n");
    content.push_str(&format!(
        "    ports:\n      - \"{}:80\"\n",
        registry.caddy_port
    ));
    content.push_str("    volumes:\n");
    content.push_str("      - ./Caddyfile:/etc/caddy/Caddyfile:ro\n");
    content.push_str("    networks:\n");
    content.push_str("      gradesta-local:\n");
    content.push_str("        aliases:\n");
    content.push_str("          - gradesta-local-services\n");
    content.push_str("    depends_on:\n");

    // Caddy depends on all enabled services
    for service in &registry.services {
        if service.enabled {
            content.push_str(&format!("      - {}\n", service.name));
        }
    }

    content.push('\n');

    // Service containers
    for service in &registry.services {
        if !service.enabled {
            continue;
        }

        // Use parent directory as build context to handle local path dependencies
        let source_path = std::path::Path::new(&service.source_dir);
        let parent_dir = source_path.parent().unwrap_or(source_path);
        let service_dirname = source_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&service.name);

        content.push_str(&format!("  {}:\n", service.name));
        content.push_str("    build:\n");
        content.push_str(&format!("      context: {}\n", parent_dir.display()));
        content.push_str(&format!("      dockerfile: {}/Dockerfile\n", service_dirname));
        content.push_str("    networks:\n");
        content.push_str("      gradesta-local:\n");
        content.push_str("        aliases:\n");
        content.push_str("          - gradesta-local-services\n");
        content.push('\n');
    }

    std::fs::write(&compose_path, content)?;

    Ok(())
}
