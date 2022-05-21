use std::path::Path;

use sled_bincode::{Batch, Db, Error as SledBinError, Key, Transactional, Tree, TreeEntry, Value};

use crate::result::Result;
use crate::types::{Entry, EntryId, FeedId, Subscription, Tagging, TaggingId};

pub struct Repo {
    db: Db,
    subs: Tree<SubscriptionEntry>,
    unread: Tree<MarkedEntry>,
    starred: Tree<MarkedEntry>,
    entries: Tree<FeedEntry>,
    taggings: Tree<TaggingEntry>,
}

impl Repo {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled_bincode::open(path)?;

        let repo = Repo {
            subs: Tree::open(&db, "subs").unwrap(),
            unread: Tree::open(&db, "unread").unwrap(),
            starred: Tree::open(&db, "starred").unwrap(),
            entries: Tree::open(&db, "entries").unwrap(),
            taggings: Tree::open(&db, "taggings").unwrap(),
            db,
        };
        Ok(repo)
    }

    pub fn get_unread(&self) -> Result<Vec<Key<MarkedEntry>>> {
        Ok(self.unread.iter().keys().collect::<Result<_, SledBinError>>()?)
    }

    pub fn add_unread<I: IntoIterator<Item = EntryId>>(&self, entries: I) -> Result<()> {
        let mut batch = Batch::default();
        for entry in entries {
            batch.insert(&entry, &())?;
        }
        self.unread.apply_batch(batch)?;
        Ok(())
    }

    pub fn delete_unread<I: IntoIterator<Item = EntryId>>(&self, entries: I) -> Result<()> {
        let mut batch = Batch::default();
        for entry in entries {
            batch.remove(&entry)?;
        }
        self.unread.apply_batch(batch)?;
        Ok(())
    }

    pub fn get_starred(&self) -> Result<Vec<Key<MarkedEntry>>> {
        Ok(self.starred.iter().keys().collect::<Result<_, SledBinError>>()?)
    }

    pub fn add_starred<I: IntoIterator<Item = EntryId>>(&self, entries: I) -> Result<()> {
        let mut batch = Batch::default();
        for entry in entries {
            batch.insert(&entry, &())?;
        }
        self.starred.apply_batch(batch)?;
        Ok(())
    }

    pub fn delete_starred<I: IntoIterator<Item = EntryId>>(&self, entries: I) -> Result<()> {
        let mut batch = Batch::default();
        for entry in entries {
            batch.remove(&entry)?;
        }
        self.starred.apply_batch(batch)?;
        Ok(())
    }

    pub fn get_entries(
        &self,
        page: usize,
        per_page: usize,
        tags: &[String],
    ) -> Result<Vec<Value<FeedEntry>>> {
        let res = if !tags.is_empty() {
            let feeds = self.get_feeds_by_tags(tags)?;
            let filter_by_feeds = |res: &Value<FeedEntry>| -> bool {
                matches!(res.value(), Ok(entry) if feeds.contains(&entry.feed_id))
            };

            self.entries
                .iter()
                .values()
                .rev()
                .filter(|res| res.as_ref().map_or(false, filter_by_feeds))
                .skip(per_page * (page.max(1) - 1))
                .take(per_page)
                .collect::<Result<Vec<_>, _>>()?
        } else {
            self.entries
                .iter()
                .values()
                .rev()
                .skip(per_page * (page.max(1) - 1))
                .take(per_page)
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(res)
    }

    pub fn get_starred_entries(&self, page: usize, per_page: usize) -> Result<Vec<Value<FeedEntry>>> {
        let res = self
            .starred
            .iter()
            .keys()
            .rev()
            .skip(per_page * (page.max(1) - 1))
            .take(per_page)
            .map(|res| self.entries.get(&res?.key()?))
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub fn get_subscriptions(&self) -> Result<Vec<Value<SubscriptionEntry>>> {
        Ok(self.subs.iter().values().collect::<Result<_, SledBinError>>()?)
    }

    pub fn new_feed_id(&self) -> Result<FeedId> {
        Ok(FeedId(self.db.generate_id()?))
    }

    pub fn add_subscription(&self, sub: &Subscription) -> Result<()> {
        self.subs.insert(&sub.feed_id, sub)?;
        Ok(())
    }

    pub fn delete_subscription(&self, id: FeedId) -> Result<()> {
        self.subs.remove(&id)?;
        Ok(())
    }

    pub fn insert_entry(&self, entry: Entry) -> Result<()> {
        (&self.entries, &self.unread).transaction(|entries, unread| {
            if entries.insert(&entry.id, &entry)?.is_none() {
                unread.insert(&entry.id, &())?;
            }
            Ok(())
        })?;
        Ok(())
    }

    pub fn get_taggings(&self) -> Result<Vec<Value<TaggingEntry>>> {
        let res = self
            .taggings
            .iter()
            .values()
            .collect::<Result<_, SledBinError>>()?;
        Ok(res)
    }

    pub fn new_tagging_id(&self) -> Result<TaggingId> {
        Ok(TaggingId(self.db.generate_id()?))
    }

    pub fn add_tagging(&self, tagging: &Tagging) -> Result<()> {
        self.taggings.insert(&tagging.id, tagging)?;
        Ok(())
    }

    pub fn delete_tagging(&self, id: TaggingId) -> Result<()> {
        self.taggings.remove(&id)?;
        Ok(())
    }

    fn get_feeds_by_tags(&self, tags: &[String]) -> Result<Vec<FeedId>> {
        let mut feeds = vec![];
        for tagging in self.taggings.iter().values() {
            let tagging = tagging?;
            let tagging = tagging.value()?;
            if tags.iter().any(|str| str == tagging.name) {
                feeds.push(tagging.feed_id);
            }
        }
        Ok(feeds)
    }
}

#[derive(Debug, Default)]
pub struct SubscriptionEntry;

impl<'a> TreeEntry<'a> for SubscriptionEntry {
    type Key = FeedId;
    type Val = Subscription<'a>;
}

#[derive(Debug, Default)]
pub struct MarkedEntry;

impl<'a> TreeEntry<'a> for MarkedEntry {
    type Key = EntryId;
    type Val = ();
}

#[derive(Debug, Default)]
pub struct FeedEntry;

impl<'a> TreeEntry<'a> for FeedEntry {
    type Key = EntryId;
    type Val = Entry<'a>;
}

#[derive(Debug, Default)]
pub struct TaggingEntry;

impl<'a> TreeEntry<'a> for TaggingEntry {
    type Key = TaggingId;
    type Val = Tagging<'a>;
}
