use derive_new::new;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;

use anyhow::{anyhow, Context};

#[derive(new, PartialEq, PartialOrd, Default, Debug, Serialize, Deserialize, Clone)]
pub struct Screencast {
    #[serde(skip)]
    pub id: String,
    #[serde(default, skip_serializing_if = "is_default")]
    #[new(value = "None")]
    pub video_file: Option<String>,
    #[serde(default = "Vec::new", skip_serializing_if = "is_default")]
    #[new(value = "vec![]")]
    pub authors: Vec<String>,
    #[serde(default = "Vec::new", skip_serializing_if = "is_default")]
    #[new(value = "vec![]")]
    pub tasks: Vec<String>, // List of task ids
    #[serde(default, skip_serializing_if = "is_default")]
    #[new(value = "0.0")]
    pub duration_seconds: f64,
    #[serde(default, skip_serializing_if = "is_default")]
    #[new(value = "None")]
    pub url: Option<String>,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

pub async fn load_metadata(metadata_path: &str) -> anyhow::Result<Screencast> {
    let mut contents = vec![];
    fs::File::open(metadata_path)
        .await
        .context(format!("Could not read {}", metadata_path))?
        .read_to_end(&mut contents)
        .await?;
    let content_string = String::from_utf8(contents)?;
    let mut metadata: Screencast = serde_yaml::from_str(&content_string)?;
    metadata.id = Path::new(metadata_path)
        .file_stem()
        .ok_or(anyhow!(
            "Could not parse path to screencast metadata file {}.",
            metadata_path
        ))?
        .to_str()
        .ok_or(anyhow!(
            "Could not parse path to screencast metadata file {}.",
            metadata_path
        ))?
        .to_string();
    Ok(metadata.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_load_metadata() {
        let mut metadata_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        metadata_path.push("test-data/screencast-metadata.yaml");
        let metadata = load_metadata(&metadata_path.to_string_lossy())
            .await
            .unwrap();
        assert_eq!(metadata.id, "screencast-metadata");
        assert_eq!(
            metadata.authors,
            vec!["Timothy Hobbs <tim@gradesta.com>"]
        );
        assert_eq!(metadata.tasks, vec!["100", "200"]);
        assert_eq!(metadata.duration_seconds, 3.4);
        assert_eq!(metadata.url, Some("https://assets.gradesta.com/screencasts/2022-8-29-52456480-87f8-4221-9587-04fd3283ab43.mp4".to_string()));
    }
}
