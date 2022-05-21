use futures_util::future::join_all;
use rsst::client::{RssClient, RssRequest};
use rsst::feed::Feed;
use time::OffsetDateTime;

use crate::repo::Repo;
use crate::result::Result;
use crate::types::{Entry, FeedId};

pub async fn refresh_all_feeds(repo: &Repo) -> Result<()> {
    tracing::info!("refreshing all subscriptions");

    let client = RssClient::default();
    let mut tasks = vec![];
    for res in repo.get_subscriptions()? {
        let sub = res.value()?;
        let task = client.exec(RssRequest::new(sub.feed_url)?);
        tasks.push(async move { task.await.map(|res| (sub.feed_id, res)) })
    }
    for res in join_all(tasks).await {
        match res {
            Ok((feed_id, resp)) => refresh_feed(repo, feed_id, resp.borrow_feed())?,
            Err(err) => tracing::error!("failed to retrieve a feed: {err}"),
        }
    }
    Ok(())
}

pub fn refresh_feed(repo: &Repo, id: FeedId, feed: &Feed<'_>) -> Result<()> {
    let created_at = OffsetDateTime::now_utc();
    for item in &feed.channel.items {
        if let Some(entry) = Entry::from_item(id, item, created_at) {
            repo.insert_entry(entry)?;
        }
    }
    Ok(())
}
