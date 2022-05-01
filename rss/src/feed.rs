#![allow(unused_must_use)]
use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use strong_xml::{XmlRead, XmlWrite};
use time::{format_description, OffsetDateTime};

#[derive(Debug, XmlWrite, XmlRead)]
#[xml(tag = "rss")]
pub struct Feed<'a> {
    #[xml(child = "channel")]
    pub channel: Channel<'a>,
}

#[derive(Debug, XmlWrite, XmlRead)]
#[xml(tag = "channel")]
pub struct Channel<'a> {
    #[xml(flatten_text = "title")]
    pub title: Cow<'a, str>,
    #[xml(flatten_text = "link")]
    pub link: Cow<'a, str>,
    #[xml(flatten_text = "description")]
    pub description: Cow<'a, str>,
    #[xml(flatten_text = "language")]
    pub language: Option<Cow<'a, str>>,
    #[xml(child = "item")]
    pub items: Vec<Item<'a>>,
}

#[derive(Debug, XmlWrite, XmlRead)]
#[xml(tag = "item")]
pub struct Item<'a> {
    #[xml(flatten_text = "title")]
    pub title: Option<Cow<'a, str>>,
    #[xml(flatten_text = "link")]
    pub link: Option<Cow<'a, str>>,
    #[xml(flatten_text = "description")]
    pub description: Option<Cow<'a, str>>,
    #[xml(flatten_text = "author")]
    pub author: Option<Cow<'a, str>>,
    #[xml(child = "enclosure")]
    pub enclosure: Option<Enclosure<'a>>,
    #[xml(child = "guid")]
    pub guid: Option<Guid<'a>>,
    #[xml(flatten_text = "pubDate")]
    pub pub_date: Option<PubDate>,
    #[xml(flatten_text = "content")]
    pub content: Option<Cow<'a, str>>,
    #[xml(flatten_text = "content:encoded")]
    pub content_encoded: Option<Cow<'a, str>>,
    #[xml(child = "media:content")]
    pub media_content: Vec<MediaContent<'a>>,
}

#[derive(Debug, XmlWrite, XmlRead)]
#[xml(tag = "guid")]
pub struct Guid<'a> {
    #[xml(text)]
    pub value: Cow<'a, str>,
    #[xml(attr = "isPermaLink")]
    pub is_perma_link: bool,
}

#[derive(Debug, XmlWrite, XmlRead)]
#[xml(tag = "enclosure")]
pub struct Enclosure<'a> {
    #[xml(attr = "url")]
    pub url: Cow<'a, str>,
    #[xml(attr = "length")]
    pub length: u32,
    #[xml(attr = "type")]
    pub mime_type: Cow<'a, str>,
}

#[derive(Debug, XmlWrite, XmlRead)]
#[xml(tag = "media:content")]
pub struct MediaContent<'a> {
    #[xml(attr = "url")]
    pub url: Cow<'a, str>,
    #[xml(attr = "type")]
    pub mime_type: Option<Cow<'a, str>>,
    #[xml(attr = "medium")]
    pub medium: Option<ContentMedium>,
}

#[derive(Debug, Clone)]
pub struct PubDate(OffsetDateTime);

impl From<PubDate> for OffsetDateTime {
    fn from(date: PubDate) -> Self {
        date.0
    }
}

impl FromStr for PubDate {
    type Err = time::error::Parse;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        let res = time::OffsetDateTime::parse(&str, &format_description::well_known::Rfc2822)?;
        Ok(PubDate(res))
    }
}

impl fmt::Display for PubDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = self
            .0
            .format(&format_description::well_known::Rfc2822)
            .map_err(|_| fmt::Error)?;
        f.write_str(&res)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMedium {
    Image,
    Audio,
    Video,
    Document,
    Executable,
}

impl FromStr for ContentMedium {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "image" => Ok(Self::Image),
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            "document" => Ok(Self::Document),
            "executable" => Ok(Self::Executable),
            _ => Err(ParseError),
        }
    }
}

impl fmt::Display for ContentMedium {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentMedium::Image => f.write_str("image"),
            ContentMedium::Audio => f.write_str("audio"),
            ContentMedium::Video => f.write_str("video"),
            ContentMedium::Document => f.write_str("document"),
            ContentMedium::Executable => f.write_str("executable"),
        }
    }
}

#[derive(Debug)]
pub struct ParseError;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("parse error")
    }
}

impl std::error::Error for ParseError {}
