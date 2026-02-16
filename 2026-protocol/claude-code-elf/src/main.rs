//! Claude Code Elf
//!
//! An elf that integrates Claude Code with Gradesta, allowing users to
//! summon AI-powered coding assistance directly from the graph.
//!
//! # Commands
//!
//! - `read-config`: Parse the cursor cell as TOML config for Claude Code
//! - `code`: Execute Claude Code with the region content as prompt

mod config;
mod docker;
mod elf;

use clap::Parser;
use gradesta_elf::ElfServer;

use crate::elf::ClaudeCodeElf;

/// Claude Code Elf - AI-powered coding assistant for Gradesta
#[derive(Parser, Debug)]
#[command(name = "claude-code-elf")]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 9000)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    println!("Claude Code Elf v{}", env!("CARGO_PKG_VERSION"));
    println!("Starting server on port {}", args.port);

    let elf = ClaudeCodeElf::new();
    let server = ElfServer::new(elf, args.port);
    server.run().await?;

    Ok(())
}
