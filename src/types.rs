use std::num::ParseIntError;
use std::str::FromStr;

use rsst::feed::{ContentMedium, Feed, Item};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::codecs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription<'a> {
    pub id: FeedId,
    #[serde(with = "codecs::rfc3339_date")]
    pub created_at: OffsetDateTime,
    pub feed_id: FeedId,
    pub title: &'a str,
    pub feed_url: &'a str,
    pub site_url: &'a str,
}

impl<'a> Subscription<'a> {
    pub fn from_feed(id: FeedId, feed: &Feed<'a>, feed_url: &'a str, created_at: OffsetDateTime) -> Self {
        Subscription {
            id,
            feed_id: id,
            title: feed.channel.title,
            feed_url,
            site_url: feed.channel.link,
            created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry<'a> {
    pub id: EntryId,
    pub feed_id: FeedId,
    pub title: Option<&'a str>,
    pub url: Option<&'a str>,
    pub extracted_content_url: Option<&'a str>,
    pub author: Option<&'a str>,
    pub content: Option<&'a str>,
    pub summary: Option<&'a str>,
    #[serde(with = "codecs::rfc3339_date")]
    pub published: OffsetDateTime,
    #[serde(with = "codecs::rfc3339_date")]
    pub created_at: OffsetDateTime,
    #[serde(rename = "images")]
    pub image: Option<Image<'a>>,
}

impl<'a> Entry<'a> {
    pub fn from_item(feed_id: FeedId, item: &Item<'a>, created_at: OffsetDateTime) -> Option<Self> {
        let ident = item.guid.as_ref().map(|guid| guid.value).or(item.link)?;
        let published = item
            .pub_date
            .clone()
            .map(Into::into)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let id = EntryId::from_ident_and_date(ident, published);
        let content = item.content_encoded.or(item.content);
        let image = item
            .media
            .iter()
            .find_map(|media| {
                media.url.filter(|_| {
                    media.medium == Some(ContentMedium::Image)
                        || media.mime_type.filter(|str| str.starts_with("image/")).is_some()
                })
            })
            .map(|url| Image { url });

        let res = Entry {
            id,
            feed_id,
            title: item.title,
            url: item.link,
            extracted_content_url: None,
            author: item.author,
            content,
            summary: item.description,
            published,
            created_at,
            image,
        };
        Some(res)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tagging<'a> {
    pub id: TaggingId,
    pub feed_id: FeedId,
    pub name: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image<'a> {
    #[serde(rename = "original_url")]
    pub url: &'a str,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TaggingId(u64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeedId(u64);

impl FromStr for FeedId {
    type Err = ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(FeedId)
    }
}

impl FeedId {
    pub fn generate(db: &sled_bincode::Db) -> Result<Self, sled_bincode::SledError> {
        db.generate_id().map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntryId(u64);

impl EntryId {
    // the date is stored as a unix timestamp in the first 4 bytes of the ID
    // this makes it possible to use the ID for sorting
    pub fn from_ident_and_date(name: &str, date: OffsetDateTime) -> Self {
        let mut bytes = [0; 0x8];
        bytes[0..4].copy_from_slice(&((date.unix_timestamp() / 1000) as u32).to_be_bytes());
        bytes[4..6].copy_from_slice(&fletcher16(name.as_bytes()).to_be_bytes());
        EntryId(u64::from_ne_bytes(bytes))
    }
}

fn fletcher16(bytes: &[u8]) -> u16 {
    let mut sum1 = 0;
    let mut sum2 = 0;

    for byte in bytes {
        sum1 = (sum1 + *byte as u16) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    (sum2 << 8) | sum1
}
