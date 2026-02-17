//! Pig Latin Elf
//!
//! A simple deterministic elf for testing. Transforms text content to pig latin.

mod transform;

use async_trait::async_trait;
use clap::Parser;
use gradesta_elf::{Elf, ElfCommand, ElfContext, ElfManifest, ElfServer};

use transform::to_pig_latin;

#[derive(Parser, Debug)]
#[command(name = "pig-latin-elf")]
#[command(about = "Pig Latin transformation elf")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 9001)]
    port: u16,
}

struct PigLatinElf;

#[async_trait]
impl Elf for PigLatinElf {
    fn manifest(&self) -> ElfManifest {
        ElfManifest {
            elf_id: "pig-latin".to_string(),
            name: "Pig Latin".to_string(),
            description: "Transforms text to pig latin".to_string(),
            commands: vec![ElfCommand {
                name: "transform".to_string(),
                description: "Transform cursor cell to pig latin".to_string(),
                inputs: vec!["cursor".to_string()],
            }],
        }
    }

    async fn handle_summon(&self, ctx: ElfContext) -> anyhow::Result<()> {
        ctx.output_line("Reading cursor vertex...").await?;

        // Read the cursor vertex content
        let (mime, content) = ctx.read_cursor().await?;

        // Check if it's text
        if !mime.starts_with("text/") {
            ctx.output_line(&format!("Cursor content is not text ({})", mime)).await?;
            ctx.complete(1, "Not text content").await?;
            return Ok(());
        }

        // Convert to string
        let text = String::from_utf8_lossy(&content);
        ctx.output_line(&format!("Original: {}", text.trim())).await?;

        // Transform to pig latin
        let pig_latin = to_pig_latin(&text);
        ctx.output_line(&format!("Transformed: {}", pig_latin.trim())).await?;

        // Write back
        ctx.set_cursor(&mime, pig_latin.as_bytes()).await?;

        ctx.output_line("Done!").await?;
        ctx.complete(0, "Transformation complete").await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    println!("Starting Pig Latin Elf on port {}", args.port);

    let server = ElfServer::new(PigLatinElf, args.port);
    server.run().await?;

    Ok(())
}
