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
            title: feed.channel.title.into(),
            feed_url,
            site_url: feed.channel.link.into(),
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
        let content = item.content.or(item.description);
        let image = item
            .media
            .iter()
            .find_map(|media| {
                media.url.filter(|_| {
                    media.medium == Some(ContentMedium::Image)
                        || media.mime_type.filter(|str| str.starts_with("image/")).is_some()
                })
            })
            .map(|url| Image {
                url: Cow::Borrowed(url),
            });

        Entry {
            id,
            feed_id,
            title: item.title.map(Cow::Borrowed),
            url: item.link.map(Cow::Borrowed),
            extracted_content_url: None,
            author: item.author.map(Cow::Borrowed),
            content: content.map(Cow::Borrowed),
            summary: item.description.map(Cow::Borrowed),
            published: item
                .pub_date
                .clone()
                .map(Into::into)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            created_at,
            image,
        }
    }

    pub fn key(&self) -> EntryKey {
        EntryKey {
            published: self.published,
            id: self.id,
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
        const MAX_JS_INT: u64 = (1 << 53) - 1;
        let hash = fnv1a64(str.as_bytes());
        EntryId(hash % MAX_JS_INT)
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
    hash
}

// enables us to sort entries by publish time
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EntryKey {
    #[serde(with = "codecs::rfc3339_date")]
    published: OffsetDateTime,
    id: EntryId,
}
