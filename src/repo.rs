use std::sync::Arc;

use futures_util::future::join_all;
use rsst::client::RssRequest;
use rsst::feed::Feed;
use sled_bincode::{ConflictableTransactionError, Transactional, Tree};
use time::OffsetDateTime;

use crate::error::ServiceEror;
use crate::types::{Entry, EntryId, FeedId, Subscription};

#[derive(Debug, Default)]
pub struct SubscriptionEntry;

impl<'a> sled_bincode::Entry<'a> for SubscriptionEntry {
    type Key = FeedId;
    type Val = Subscription<'a>;
}

#[derive(Debug, Default)]
pub struct MarkedEntry;

impl<'a> sled_bincode::Entry<'a> for MarkedEntry {
    type Key = EntryId;
    type Val = ();
}

#[derive(Debug, Default)]
pub struct FeedEntry;

impl<'a> sled_bincode::Entry<'a> for FeedEntry {
    type Key = EntryId;
    type Val = Entry<'a>;
}

pub struct Repo {
    pub db: sled_bincode::Db,
    pub subs: Tree<SubscriptionEntry>,
    pub unread: Tree<MarkedEntry>,
    pub starred: Tree<MarkedEntry>,
    pub entries: Tree<FeedEntry>,
}

pub async fn trigger_refresh_all_subsripions(repo: Arc<Repo>) {
    if let Err(err) = refresh_all_subsripions(repo).await {
        tracing::error!("subscription refresh failed: {err}");
    }
}

pub async fn refresh_all_subsripions(repo: Arc<Repo>) -> Result<(), ServiceEror> {
    tracing::info!("refreshing all subscriptions");

    let mut tasks = vec![];
    for res in repo.subs.iter().values() {
        let sub = res?;
        let sub = sub.value()?;
        let req = RssRequest::new(sub.feed_url)?;
        tasks.push(async move { req.exec().await.map(|res| (sub.feed_id, res)) });
    }
    for res in join_all(tasks).await {
        match res {
            Ok((feed_id, feed)) => refresh_feed(repo.clone(), feed_id, feed.borrow_feed()).await?,
            Err(err) => tracing::error!("failed to retrieve a feed: {err}"),
        }
    }
    Ok(())
}

pub async fn refresh_feed(repo: Arc<Repo>, feed_id: FeedId, rss: &Feed<'_>) -> Result<(), ServiceEror> {
    (&repo.entries, &repo.unread).transaction(|entries, unread| {
        let created_at = OffsetDateTime::now_utc();

        for item in &rss.channel.items {
            if let Some(entry) = Entry::from_item(feed_id, item, created_at) {
                if entries.insert(&entry.id, &entry)?.is_none() {
                    unread.insert(&entry.id, &())?;
                }
            }
        }

        Ok::<(), ConflictableTransactionError>(())
    })?;

    Ok(())
}
