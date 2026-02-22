use crate::config::{services_dir, ServiceRegistry};
use anyhow::Result;

pub fn generate_caddyfile(registry: &ServiceRegistry) -> Result<()> {
    let services_dir = services_dir()?;
    let caddyfile_path = services_dir.join("Caddyfile");

    let mut content = String::new();

    content.push_str(":80 {\n");

    for service in &registry.services {
        if !service.enabled {
            continue;
        }

        content.push_str(&format!("    handle /{}/* {{\n", service.name));
        content.push_str(&format!("        uri strip_prefix /{}\n", service.name));
        content.push_str(&format!(
            "        reverse_proxy {}:{}\n",
            service.name, service.internal_port
        ));
        content.push_str("    }\n\n");
    }

    // Default response for root
    content.push_str("    handle {\n");
    content.push_str("        respond \"Gradesta Service Manager\" 200\n");
    content.push_str("    }\n");

    content.push_str("}\n");

    std::fs::write(&caddyfile_path, content)?;

    Ok(())
}
