use super::config::Config;
use super::fetch_duration::fetch_duration;
use super::screencast::Screencast;

pub async fn save_screencast_metadata(
    screencasts: &mut Vec<Screencast>,
    config: &Config,
) -> anyhow::Result<()> {
    for i in 0..screencasts.len() {
        {
            let mut screencast = &mut screencasts[i];
            let video_file = &screencast.video_file.as_ref().unwrap().clone();
            fetch_duration(&mut screencast, &video_file).await?;
        }
        let mut screencast = &mut screencasts[i];
        screencast.url = Some(format!(
            "{}{}.mp4",
            config.screencasts_base_url, screencast.id
        ));
        let mut metadata_path = config.git_root.clone();
        metadata_path.push("screencasts/");
        metadata_path.push(&screencast.id);
        metadata_path.set_extension("yaml");
        let f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(metadata_path)
            .expect("Couldn't open file");
        serde_yaml::to_writer(f, &screencast)?;
    }
    Ok(())
}
