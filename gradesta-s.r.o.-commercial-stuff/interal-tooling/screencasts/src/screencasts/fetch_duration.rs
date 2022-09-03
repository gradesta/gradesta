use super::screencast::load_metadata;

use anyhow::{anyhow, Context};
use tokio::process::Command;

pub async fn fetch_duration(metadata_path: &str, missing: bool) -> anyhow::Result<()> {
    let mut screencast = load_metadata(metadata_path).await?;
    if missing && screencast.duration_seconds > 0.0 {
        println!("Duration already set for {}", metadata_path);
        return Ok(());
    }
    println!("Fetching duration data data for {}", metadata_path);
    let url: String = screencast.url.clone().ok_or(anyhow!(
        "Metadata file {} missing URL, cannot fetch duration.",
        metadata_path
    ))?;
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(&url)
        .output()
        .await
        .context("Error running ffmpeg command")?;
    let ffmpeg_output0 = String::from_utf8(output.stdout)?;
    let ffmpeg_output = ffmpeg_output0.trim();
    screencast.duration_seconds = ffmpeg_output
        .parse::<f64>()
        .context(format!("Could not parse {}", ffmpeg_output))?;
    println!(
        "{} fetched, duration is {} seconds.",
        &url, screencast.duration_seconds
    );
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(metadata_path)
        .expect("Couldn't open file");
    serde_yaml::to_writer(f, &screencast)?;
    Ok(())
}
