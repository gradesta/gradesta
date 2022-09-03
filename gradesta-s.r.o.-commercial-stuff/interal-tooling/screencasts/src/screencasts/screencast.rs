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
