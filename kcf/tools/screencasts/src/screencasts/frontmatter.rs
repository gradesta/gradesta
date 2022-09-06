use derive_new::new;
use serde::{de, Deserialize, Deserializer};
use time::{format_description, Date};

#[derive(new, PartialEq, PartialOrd, Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct FrontMatter {
    pub title: String,
    #[serde(deserialize_with = "deserialize_yyyy_mm_dd_date")]
    pub date: Date,
    #[serde(default)]
    pub featureImage: Option<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub authors: Vec<String>,
}

fn deserialize_yyyy_mm_dd_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let format = format_description::parse("[year]-[month]-[day]").map_err(de::Error::custom)?;
    let dtfo = Date::parse(&s, &format).map_err(de::Error::custom)?;
    Ok(dtfo.into())
}
