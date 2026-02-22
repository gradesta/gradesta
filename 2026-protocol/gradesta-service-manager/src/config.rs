use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    #[serde(rename = "type")]
    pub service_type: String,
    pub source_dir: String,
    pub internal_port: u16,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistry {
    pub caddy_port: u16,
    #[serde(default)]
    pub services: Vec<Service>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self {
            caddy_port: 19333,
            services: Vec::new(),
        }
    }
}

pub fn services_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let services_dir = config_dir.join("gradesta").join("services");
    std::fs::create_dir_all(&services_dir)?;
    Ok(services_dir)
}

impl ServiceRegistry {
    pub fn load() -> Result<Self> {
        let registry_path = services_dir()?.join("registry.toml");

        if registry_path.exists() {
            let content = std::fs::read_to_string(&registry_path)?;
            let registry: ServiceRegistry = toml::from_str(&content)?;
            Ok(registry)
        } else {
            Ok(ServiceRegistry::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let registry_path = services_dir()?.join("registry.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&registry_path, content)?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct BrowserServices {
    pub caddy_url: String,
    pub servers: Vec<BrowserService>,
    pub elves: Vec<BrowserService>,
}

#[derive(Debug, Serialize)]
pub struct BrowserService {
    pub name: String,
    pub url: String,
}

pub fn generate_browser_services(registry: &ServiceRegistry) -> Result<()> {
    let services_dir = services_dir()?;
    let browser_services_path = services_dir.join("browser-services.json");

    let caddy_url = format!("http://localhost:{}", registry.caddy_port);
    let caddy_ws_url = format!("ws://localhost:{}", registry.caddy_port);

    let mut servers = Vec::new();
    let mut elves = Vec::new();

    for service in &registry.services {
        if !service.enabled {
            continue;
        }

        if service.service_type == "elf" {
            // Elves use HTTP for REST API
            elves.push(BrowserService {
                name: service.name.clone(),
                url: format!("{}/{}", caddy_url, service.name),
            });
        } else {
            // Servers use WebSocket
            servers.push(BrowserService {
                name: service.name.clone(),
                url: format!("{}/{}", caddy_ws_url, service.name),
            });
        }
    }

    let browser_services = BrowserServices {
        caddy_url,
        servers,
        elves,
    };

    let content = serde_json::to_string_pretty(&browser_services)?;
    std::fs::write(&browser_services_path, content)?;

    Ok(())
}
