use std::borrow::Cow;

use rss::feed::{ContentMedium, Feed, Item};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::codecs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription<'a> {
    pub id: FeedId,
    #[serde(with = "codecs::rfc3339_date")]
    pub created_at: OffsetDateTime,
    pub feed_id: FeedId,
    pub title: Cow<'a, str>,
    pub feed_url: Cow<'a, str>,
    pub site_url: Cow<'a, str>,
}

impl<'a> Subscription<'a> {
    pub fn from_feed(
        id: FeedId,
        feed: &Feed<'a>,
        feed_url: Cow<'a, str>,
        created_at: OffsetDateTime,
    ) -> Self {
        Subscription {
            id,
            feed_id: id,
            title: feed.channel.title.clone(),
            feed_url,
            site_url: feed.channel.link.clone(),
            created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry<'a> {
    pub id: EntryId,
    pub feed_id: FeedId,
    pub title: Option<Cow<'a, str>>,
    pub url: Option<Cow<'a, str>>,
    pub extracted_content_url: Option<Cow<'a, str>>,
    pub author: Option<Cow<'a, str>>,
    pub content: Option<Cow<'a, str>>,
    pub summary: Option<Cow<'a, str>>,
    #[serde(with = "codecs::rfc3339_date")]
    pub published: OffsetDateTime,
    #[serde(with = "codecs::rfc3339_date")]
    pub created_at: OffsetDateTime,
    #[serde(rename = "images")]
    pub image: Option<Image<'a>>,
}

impl<'a> Entry<'a> {
    pub fn from_item(id: EntryId, feed_id: FeedId, item: &Item<'a>, created_at: OffsetDateTime) -> Self {
        let content = item
            .content
            .as_ref()
            .or(item.content_encoded.as_ref())
            .or(item.description.as_ref())
            .cloned();
        let image = item
            .media_content
            .iter()
            .find(|media| {
                let mime = media.mime_type.as_deref();
                media.medium == Some(ContentMedium::Image)
                    || mime.filter(|str| str.starts_with("image/")).is_some()
            })
            .map(|media| Image {
                url: media.url.clone(),
            });

        Entry {
            id,
            feed_id,
            title: item.title.clone(),
            url: item.link.clone(),
            extracted_content_url: None,
            author: item.author.clone(),
            content,
            summary: item.description.clone(),
            published: item.pub_date.clone().map(Into::into).unwrap_or(created_at),
            created_at,
            image,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tagging<'a> {
    pub id: TaggingId,
    pub feed_id: FeedId,
    pub name: Cow<'a, str>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image<'a> {
    #[serde(rename = "original_url")]
    pub url: Cow<'a, str>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TaggingId(u64);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeedId(u64);

impl FeedId {
    pub fn generate(db: &sled::Db) -> Result<Self, sled::Error> {
        db.generate_id().map(Self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntryId(u64);

impl EntryId {
    pub fn from_ident(str: &str) -> Self {
        EntryId(fnv1a64(str.as_bytes()))
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x100000001b3;
    const SEED: u64 = 0xCBF29CE484222325;

    let mut hash = SEED;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    (hash as u32) as u64
}
