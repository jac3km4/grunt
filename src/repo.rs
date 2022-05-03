use std::sync::Arc;

use futures_util::future::join_all;
use rsst::client::RssRequest;
use rsst::feed::Feed;
use sled::transaction::ConflictableTransactionError;
use time::OffsetDateTime;
use typed_sled::Tree;

use crate::error::ServiceEror;
use crate::types::{Entry, EntryId, EntryKey, FeedId, Subscription};

pub struct Repo<'a> {
    pub db: sled::Db,
    pub subs: Tree<FeedId, Subscription<'a>>,
    pub unread: Tree<EntryId, ()>,
    pub starred: Tree<EntryId, ()>,
    pub entries: Tree<EntryKey, Entry<'a>>,
    pub entry_ids: Tree<EntryId, EntryKey>,
}

pub async fn trigger_refresh_all_subsripions(repo: Arc<Repo<'_>>) {
    if let Err(err) = refresh_all_subsripions(repo).await {
        tracing::error!("subscription refresh failed: {err}");
    }
}

pub async fn refresh_all_subsripions(repo: Arc<Repo<'_>>) -> Result<(), ServiceEror> {
    tracing::info!("refreshing all subscriptions");

    let mut tasks = vec![];
    for res in repo.subs.iter().values() {
        let sub = res?;
        let req = RssRequest::new(&sub.feed_url)?;
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

pub async fn refresh_feed(repo: Arc<Repo<'_>>, feed_id: FeedId, rss: &Feed<'_>) -> Result<(), ServiceEror> {
    repo.entry_ids
        .transaction3(&repo.entries, &repo.unread, |entry_ids, entries, unread| {
            let created_at = OffsetDateTime::now_utc();

            for item in &rss.channel.items {
                if let Some(ident) = item.guid.as_ref().map(|guid| guid.value).or(item.link) {
                    let id = EntryId::from_ident(ident);
                    let entry = Entry::from_item(id, feed_id, item, created_at);
                    let key = entry.key();
                    if entries.insert(&key, &entry)?.is_none() {
                        unread.insert(&id, &())?;
                        entry_ids.insert(&id, &key)?;
                    }
                }
            }

            Ok::<(), ConflictableTransactionError>(())
        })?;

    Ok(())
}
