use std::borrow::Cow;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::Query;
use axum::handler::Handler;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use rss::client::RssRequest;
use serde::Deserialize;
use time::OffsetDateTime;
use tower_http::auth::RequireAuthorizationLayer;
use tower_http::trace::TraceLayer;
use typed_sled::Batch;

use crate::error::ServiceEror;
use crate::repo::{self, Repo};
use crate::types::{EntryId, FeedId, Subscription};
use crate::AppConfig;

pub async fn run(repo: Arc<Repo<'static>>, config: &AppConfig) {
    let admin_api = Router::new()
        .route("/subscriptions", post(add_subscription))
        .route("/jobs/refresh", post(refresh_subscriptions));

    let feedbin_api = Router::new()
        .route("/authentication.json", get(authenticate))
        .route("/subscriptions.json", get(get_subscriptions))
        .route(
            "/unread_entries.json",
            get(get_unread).post(post_unread).delete(delete_unread),
        )
        .route(
            "/starred_entries.json",
            get(get_starred).post(post_starred).delete(delete_starred),
        )
        .route("/entries.json", get(get_entries));

    let app = Router::new()
        .nest("/admin", admin_api)
        .nest("/feedbin", feedbin_api)
        .fallback(fallback.into_service())
        .layer(TraceLayer::new_for_http())
        .layer(RequireAuthorizationLayer::basic(&config.user, &config.password))
        .layer(Extension(repo.clone()));

    tracing::info!("starting a server on port {}", config.port);
    axum::Server::bind(&([0, 0, 0, 0], config.port).into())
        .serve(app.into_make_service())
        .await
        .expect("http server failed")
}

async fn authenticate() -> StatusCode {
    // authentication is done in the middleware
    StatusCode::OK
}

async fn get_subscriptions(Extension(repo): Extension<Arc<Repo<'_>>>) -> impl IntoResponse + '_ {
    let res = repo.subs.iter().values().collect::<Result<Vec<_>, _>>()?;
    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn get_unread(Extension(repo): Extension<Arc<Repo<'_>>>) -> impl IntoResponse {
    let res = repo.unread.iter().keys().collect::<Result<Vec<_>, _>>()?;
    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn post_unread(
    Extension(repo): Extension<Arc<Repo<'_>>>,
    Json(entries): Json<UnreadEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.unread_entries {
        batch.insert(&entry, &());
    }
    repo.unread.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn delete_unread(
    Extension(repo): Extension<Arc<Repo<'_>>>,
    Json(entries): Json<UnreadEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.unread_entries {
        batch.remove(&entry);
    }
    repo.unread.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn get_starred(Extension(repo): Extension<Arc<Repo<'_>>>) -> impl IntoResponse {
    let res = repo.starred.iter().keys().collect::<Result<Vec<_>, _>>()?;
    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn post_starred(
    Extension(repo): Extension<Arc<Repo<'_>>>,
    Json(entries): Json<StarredEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.starred_entries {
        batch.insert(&entry, &());
    }
    repo.starred.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn delete_starred(
    Extension(repo): Extension<Arc<Repo<'_>>>,
    Json(entries): Json<StarredEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.starred_entries {
        batch.remove(&entry);
    }
    repo.starred.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn get_entries(
    Extension(repo): Extension<Arc<Repo<'_>>>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse + '_ {
    let res = repo
        .entries
        .iter()
        .values()
        .skip(pagination.per_page * (pagination.page - 1))
        .take(pagination.per_page)
        .collect::<Result<Vec<_>, _>>()?;

    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn add_subscription<'a>(
    Extension(repo): Extension<Arc<Repo<'_>>>,
    Json(add_sub): Json<AddSubscription<'a>>,
) -> Result<Response, ServiceEror> {
    let id = FeedId::generate(&repo.db)?;
    let created_at = OffsetDateTime::now_utc();
    let feed = RssRequest::new(&add_sub.feed_url)?.exec().await?;
    let sub = Subscription::from_feed(id, feed.borrow_feed(), add_sub.feed_url, created_at);

    repo.subs.insert(&id, &sub)?;
    repo::refresh_feed(repo.clone(), id, feed.borrow_feed()).await?;
    repo.db.flush_async().await?;

    tracing::info!("successfully added a subscription for {}", sub.feed_url);
    Ok((StatusCode::OK, Json(sub)).into_response())
}

async fn refresh_subscriptions(Extension(repo): Extension<Arc<Repo<'_>>>) -> impl IntoResponse {
    repo::refresh_all_subsripions(repo.clone()).await?;
    repo.db.flush_async().await?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn fallback(req: Request<Body>) -> impl IntoResponse {
    tracing::info!("fallback {:?}", req);
    StatusCode::NOT_FOUND
}

#[derive(Debug, Deserialize)]
struct Pagination {
    page: usize,
    per_page: usize,
}

#[derive(Debug, Deserialize)]
struct AddSubscription<'a> {
    feed_url: Cow<'a, str>,
}

#[derive(Debug, Deserialize)]
struct UnreadEntries {
    unread_entries: Vec<EntryId>,
}

#[derive(Debug, Deserialize)]
struct StarredEntries {
    starred_entries: Vec<EntryId>,
}
