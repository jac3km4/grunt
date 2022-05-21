use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{FromRequest, Path, Query, RequestParts};
use axum::handler::Handler;
use axum::http::{Method, Request, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{async_trait, Extension, Json, Router};
use rsst::client::{RssClient, RssRequest};
use serde::{Deserialize, Deserializer};
use time::OffsetDateTime;
use tower_http::auth::RequireAuthorizationLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::refresh::{refresh_all_feeds, refresh_feed};
use crate::repo::Repo;
use crate::result::{Result, ServiceEror};
use crate::types::{EntryId, FeedId, Subscription, Tagging, TaggingId};
use crate::AppConfig;

pub async fn run(repo: Arc<Repo>, config: &AppConfig) {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_origin(Any);

    let admin_api = Router::new().route("/jobs/refresh", post(refresh_subscriptions));

    let feedbin_api = Router::new()
        .route("/authentication.json", get(authenticate))
        .route(
            "/subscriptions.json",
            get(get_subscriptions).post(add_subscription),
        )
        .route("/subscriptions/:id.json", delete(delete_subscription))
        .route(
            "/unread_entries.json",
            get(get_unread).post(post_unread).delete(delete_unread),
        )
        .route(
            "/starred_entries.json",
            get(get_starred).post(post_starred).delete(delete_starred),
        )
        .route("/entries.json", get(get_entries))
        .route("/taggings.json", get(get_taggings).post(create_tagging))
        .route("/taggings/:id.json", delete(delete_tagging));

    let app = Router::new()
        .nest("/admin", admin_api)
        .nest("/feedbin", feedbin_api)
        .route("/webui", get(get_webui))
        .fallback(fallback.into_service())
        .layer(TraceLayer::new_for_http())
        .layer(RequireAuthorizationLayer::basic(&config.user, &config.password))
        .layer(cors)
        .layer(Extension(repo.clone()));

    tracing::info!("starting a server on port {}", config.port);
    axum::Server::bind(&([0, 0, 0, 0], config.port).into())
        .serve(app.into_make_service())
        .await
        .expect("http server failed")
}

async fn get_webui() -> impl IntoResponse {
    Html(include_str!("../resources/index.html"))
}

async fn authenticate() -> StatusCode {
    // authentication is done in the middleware
    StatusCode::OK
}

async fn get_subscriptions(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    repo.get_subscriptions().map(Json)
}

async fn get_unread(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    repo.get_unread().map(Json)
}

async fn post_unread(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<UnreadEntries>,
) -> impl IntoResponse {
    repo.add_unread(entries.unread_entries.iter().copied())?;
    Ok::<_, ServiceEror>(Json(entries.unread_entries))
}

async fn delete_unread(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<UnreadEntries>,
) -> impl IntoResponse {
    repo.delete_unread(entries.unread_entries)
}

async fn get_starred(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    repo.get_starred().map(Json)
}

async fn post_starred(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<StarredEntries>,
) -> impl IntoResponse {
    repo.add_starred(entries.starred_entries.iter().copied())?;
    Ok::<_, ServiceEror>(Json(entries.starred_entries))
}

async fn delete_starred(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<StarredEntries>,
) -> impl IntoResponse {
    repo.delete_starred(entries.starred_entries)
}

async fn get_entries(
    Extension(repo): Extension<Arc<Repo>>,
    Query(query): Query<EntriesQuery>,
) -> impl IntoResponse {
    if let Some(true) = query.starred {
        repo.get_starred_entries(query.page, query.per_page).map(Json)
    } else {
        repo.get_entries(query.page, query.per_page, &query.tags)
            .map(Json)
    }
}

async fn add_subscription(
    Extension(repo): Extension<Arc<Repo>>,
    Json(add_sub): Json<AddSubscription>,
) -> Result<Response, ServiceEror> {
    let created_at = OffsetDateTime::now_utc();
    let feed = RssClient::default()
        .exec(RssRequest::new(&add_sub.feed_url)?)
        .await?;
    let id = repo.new_feed_id()?;
    let sub = Subscription::from_feed(id, feed.borrow_feed(), &add_sub.feed_url, created_at);
    repo.add_subscription(&sub)?;
    refresh_feed(&repo, id, feed.borrow_feed())?;

    tracing::info!("successfully added a subscription for {}", sub.feed_url);
    Ok((StatusCode::CREATED, Json(sub)).into_response())
}

async fn delete_subscription(
    Extension(repo): Extension<Arc<Repo>>,
    PathWithExt(feed_id): PathWithExt<FeedId>,
) -> impl IntoResponse {
    repo.delete_subscription(feed_id)
}

async fn refresh_subscriptions(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    refresh_all_feeds(&repo).await
}

async fn get_taggings(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    repo.get_taggings().map(Json)
}

async fn create_tagging(
    Extension(repo): Extension<Arc<Repo>>,
    Json(add_tagging): Json<AddTagging>,
) -> Result<Response, ServiceEror> {
    let id = repo.new_tagging_id()?;
    let tagging = Tagging::new(id, add_tagging.feed_id, &add_tagging.name);
    repo.add_tagging(&tagging)?;
    Ok((StatusCode::CREATED, Json(tagging)).into_response())
}

async fn delete_tagging(
    Extension(repo): Extension<Arc<Repo>>,
    PathWithExt(tagging_id): PathWithExt<TaggingId>,
) -> impl IntoResponse {
    repo.delete_tagging(tagging_id)
}

async fn fallback(req: Request<Body>) -> impl IntoResponse {
    tracing::info!("request not matched: {}", req.uri());
    StatusCode::NOT_FOUND
}

#[derive(Debug, Deserialize)]
struct EntriesQuery {
    page: usize,
    per_page: usize,
    starred: Option<bool>,
    #[serde(deserialize_with = "deserialize_qs_array", default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AddSubscription {
    feed_url: String,
}

#[derive(Debug, Deserialize)]
struct UnreadEntries {
    unread_entries: Vec<EntryId>,
}

#[derive(Debug, Deserialize)]
struct StarredEntries {
    starred_entries: Vec<EntryId>,
}

#[derive(Debug, Deserialize)]
struct AddTagging {
    feed_id: FeedId,
    name: String,
}

struct PathWithExt<A>(A);

#[async_trait]
impl<A: FromStr, B: Send> FromRequest<B> for PathWithExt<A> {
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let str = req
            .extract::<Path<String>>()
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "could not extract path parameter"))?;
        match str.split_once('.') {
            Some((str, _)) => str
                .parse()
                .map(PathWithExt)
                .map_err(|_| (StatusCode::BAD_REQUEST, "could not parse path parameter")),
            None => return Err((StatusCode::BAD_REQUEST, "missing path parameter")),
        }
    }
}

fn deserialize_qs_array<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    <&'de str>::deserialize(deserializer)?
        .split(',')
        .map(T::from_str)
        .collect::<Result<Vec<T>, T::Err>>()
        .map_err(serde::de::Error::custom)
}
