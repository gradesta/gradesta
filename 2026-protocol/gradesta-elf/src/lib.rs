//! Gradesta Elf Library
//!
//! This library provides the framework for building elf services that can
//! be summoned by the Gradesta browser to perform tasks on the graph.
//!
//! # Overview
//!
//! An elf is an HTTP service that:
//! 1. Serves a manifest at `GET /manifest.json` describing its capabilities
//! 2. Accepts summon requests at `POST /summon`
//! 3. Connects to the server WebSocket with the provided token
//! 4. Performs operations on the graph within the granted region
//! 5. Streams output back to the browser
//!
//! # Example
//!
//! ```rust,ignore
//! use gradesta_elf::{Elf, ElfManifest, ElfCommand, ElfServer, ElfContext};
//! use async_trait::async_trait;
//!
//! struct MyElf;
//!
//! #[async_trait]
//! impl Elf for MyElf {
//!     fn manifest(&self) -> ElfManifest {
//!         ElfManifest {
//!             elf_id: "my-elf".to_string(),
//!             name: "My Elf".to_string(),
//!             description: "Does something useful".to_string(),
//!             commands: vec![
//!                 ElfCommand {
//!                     name: "greet".to_string(),
//!                     description: "Says hello".to_string(),
//!                     inputs: vec!["cursor".to_string()],
//!                 },
//!             ],
//!         }
//!     }
//!
//!     async fn handle_summon(&self, ctx: ElfContext) -> anyhow::Result<()> {
//!         ctx.output("Hello from elf!").await?;
//!         ctx.complete(0, "Done").await?;
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = ElfServer::new(MyElf, 9000);
//!     server.run().await.unwrap();
//! }
//! ```

mod context;
mod protocol;
mod server;

pub use context::ElfContext;
pub use server::ElfServer;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manifest describing an elf's capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElfManifest {
    /// Unique identifier for this elf
    pub elf_id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this elf does
    pub description: String,
    /// Available commands
    pub commands: Vec<ElfCommand>,
}

/// A command exposed by an elf
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElfCommand {
    /// Command name (identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Input types required (e.g., "cursor", "region", "prompt")
    pub inputs: Vec<String>,
}

/// Summon request sent by the browser
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummonRequest {
    /// Token to connect to the server
    pub token: String,
    /// WebSocket URL of the server
    pub server_ws_url: String,
    /// Command to execute
    pub command: String,
    /// Command parameters
    #[serde(default)]
    pub params: HashMap<String, String>,
}

/// Region specification from the task
#[derive(Clone, Debug)]
pub struct RegionSpec {
    /// Landmark containing the origin vertex
    pub origin_landmark: String,
    /// Origin vertex ID
    pub origin_vertex: u64,
    /// Allowed directions bitmask
    pub allowed_directions: u8,
    /// Maximum traversal depth
    pub max_depth: i32,
}

/// Task details received from the server
#[derive(Clone, Debug)]
pub struct ElfTask {
    /// Command to execute
    pub command: String,
    /// Cursor landmark
    pub cursor_landmark: String,
    /// Cursor vertex ID
    pub cursor_vertex: u64,
    /// Cursor vertex MIME type
    pub cursor_mime: String,
    /// Cursor vertex content (provided by server, no need to request)
    pub cursor_content: Vec<u8>,
    /// Region the elf can access
    pub region: RegionSpec,
    /// Task parameters
    pub params: HashMap<String, String>,
}

/// Trait that all elves must implement
#[async_trait]
pub trait Elf: Send + Sync {
    /// Return the elf's manifest
    fn manifest(&self) -> ElfManifest;

    /// Handle a summon request
    ///
    /// This is called when the elf receives a task from the server.
    /// Use the provided context to:
    /// - Read/write vertices in the graph
    /// - Stream output back to the browser
    /// - Signal completion
    async fn handle_summon(&self, ctx: ElfContext) -> anyhow::Result<()>;
}
