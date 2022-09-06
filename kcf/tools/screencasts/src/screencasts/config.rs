use derive_new::new;
use serde::{Deserialize, Serialize};

use anyhow::Context;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

#[derive(new, Default, Serialize, Deserialize, Debug)]
pub struct Config {
    pub s3_screencasts_path: String,
    pub screencasts_base_url: String,
    #[serde(skip)]
    #[new(value = "PathBuf::new()")]
    pub git_root: PathBuf,
}

pub async fn load_config(blogpost_path: &str) -> anyhow::Result<Config> {
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
    fs::File::open(&screencast_config_file_path)
        .await
        .context(format!("Could not read {:?}", &screencast_config_file_path))?
        .read_to_end(&mut contents)
        .await?;
    let mut config: Config = serde_yaml::from_str(&String::from_utf8(contents)?)?;
    config.git_root = PathBuf::new();
    config.git_root.push(git_repo_path);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::load_config;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_load_config() {
        let mut blogpost = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        blogpost.push("test-data/blogpost.md");
        let config = load_config(&blogpost.to_string_lossy()).await.unwrap();
        assert_eq!(
            config.s3_screencasts_path,
            "s3://gradesta-web-static/screencasts/"
        );
        assert_eq!(
            config.screencasts_base_url,
            "https://assets.gradesta.com/screencasts/"
        );
    }
}
