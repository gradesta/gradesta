
use tokio::process::Command;
use tokio::fs;
use tokio::io::AsyncReadExt;
use std::path::Path;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub id: String,
    pub author: String,
    pub url: String,
    pub duration_seconds: Option<String>,
    pub tasks: Vec<String>,
}

pub async fn load_metadata(blogpost_path: &str) -> anyhow::Result<Config> {
    let blogpost_dir = Path::new(blogpost_path).parent().unwrap_or(Path::new("./"));
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(&blogpost_dir)
        .output()
        .await?;
    let repo_path_string0 = String::from_utf8(output.stdout)?;
    let repo_path_string = repo_path_string0.trim();
    let git_repo_path = Path::new(&repo_path_string);
    let screencast_config_file_path = git_repo_path.join("screencasts.yaml");
    let mut contents = vec![];
    fs::File::open(screencast_config_file_path)
        .await?
        .read_to_end(&mut contents).await?;
    let config: Config = serde_yaml::from_str(&String::from_utf8(contents)?)?;
    Ok(config)
}
