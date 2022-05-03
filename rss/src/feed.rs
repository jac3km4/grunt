use serde::{Deserialize, Serialize};
use time::{format_description, OffsetDateTime};

#[derive(Debug, Deserialize)]
pub struct Feed<'a> {
    #[serde(borrow)]
    pub channel: Channel<'a>,
}

#[derive(Debug, Deserialize)]
pub struct Channel<'a> {
    pub title: &'a str,
    pub link: &'a str,
    pub description: Option<&'a str>,
    pub language: Option<&'a str>,
    #[serde(rename = "item", default)]
    pub items: Vec<Item<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct Item<'a> {
    pub title: Option<&'a str>,
    pub link: Option<&'a str>,
    pub description: Option<&'a str>,
    pub author: Option<&'a str>,
    pub enclosure: Option<Enclosure<'a>>,
    pub guid: Option<Guid<'a>>,
    #[serde(rename = "pubDate")]
    pub pub_date: Option<PubDate>,
    #[serde(rename = "content")]
    pub content: Option<&'a str>,
    #[serde(rename = "$ns:content", default)]
    pub media: Vec<MediaContent<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct Guid<'a> {
    #[serde(rename = "$text")]
    pub value: &'a str,
    #[serde(rename = "isPermaLink")]
    pub is_perma_link: bool,
}

#[derive(Debug, Deserialize)]
pub struct Enclosure<'a> {
    pub url: &'a str,
    pub length: u32,
    pub mime_type: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct MediaContent<'a> {
    pub url: Option<&'a str>,
    pub mime_type: Option<&'a str>,
    pub medium: Option<ContentMedium>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentMedium {
    Image,
    Audio,
    Video,
    Document,
    Executable,
}

#[derive(Debug, Clone)]
pub struct PubDate(OffsetDateTime);

impl From<PubDate> for OffsetDateTime {
    fn from(date: PubDate) -> Self {
        date.0
    }
}

impl<'de> Deserialize<'de> for PubDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = Deserialize::<'de>::deserialize(deserializer)?;
        let res = time::OffsetDateTime::parse(str, &format_description::well_known::Rfc2822)
            .map_err(serde::de::Error::custom)?;
        Ok(PubDate(res))
    }
}

impl Serialize for PubDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0
            .format(&format_description::well_known::Rfc2822)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
