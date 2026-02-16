//! HTTP server for elf services

use std::sync::Arc;

use axum::{
    extract::{Json, State},
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;

use crate::{Elf, ElfContext, SummonRequest};

/// HTTP server for an elf service
pub struct ElfServer<E: Elf + 'static> {
    elf: Arc<E>,
    port: u16,
}

impl<E: Elf + 'static> ElfServer<E> {
    /// Create a new elf server
    pub fn new(elf: E, port: u16) -> Self {
        Self {
            elf: Arc::new(elf),
            port,
        }
    }

    /// Run the server
    pub async fn run(&self) -> anyhow::Result<()> {
        let elf = Arc::clone(&self.elf);

        let app = Router::new()
            .route("/manifest.json", get(handle_manifest::<E>))
            .route("/summon", post(handle_summon::<E>))
            .with_state(elf);

        let addr = format!("0.0.0.0:{}", self.port);
        println!("Elf server listening on {}", addr);

        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle_manifest<E: Elf>(
    State(elf): State<Arc<E>>,
) -> Json<crate::ElfManifest> {
    Json(elf.manifest())
}

async fn handle_summon<E: Elf + 'static>(
    State(elf): State<Arc<E>>,
    Json(request): Json<SummonRequest>,
) -> &'static str {
    log::info!("Received summon request: command={}", request.command);

    let elf = Arc::clone(&elf);
    let token = request.token;
    let server_ws_url = request.server_ws_url;

    // Spawn task to handle the summon
    tokio::spawn(async move {
        match ElfContext::connect(&server_ws_url, &token).await {
            Ok(ctx) => {
                if let Err(e) = elf.handle_summon(ctx).await {
                    log::error!("Elf task failed: {}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to connect to server: {}", e);
            }
        }
    });

    "OK"
}
