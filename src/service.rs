use std::borrow::Cow;
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
use serde::Deserialize;
use sled_bincode::Batch;
use time::OffsetDateTime;
use tower_http::auth::RequireAuthorizationLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::error::ServiceEror;
use crate::repo::{self, Repo};
use crate::types::{EntryId, FeedId, Subscription};
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
        .route("/entries.json", get(get_entries));

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
    let res = repo.subs.iter().values().collect::<Result<Vec<_>, _>>()?;
    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn get_unread(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    let res = repo.unread.iter().keys().collect::<Result<Vec<_>, _>>()?;
    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn post_unread(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<UnreadEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.unread_entries {
        batch.insert(&entry, &())?;
    }
    repo.unread.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn delete_unread(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<UnreadEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.unread_entries {
        batch.remove(&entry)?;
    }
    repo.unread.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn get_starred(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    let res = repo.starred.iter().keys().collect::<Result<Vec<_>, _>>()?;
    Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
}

async fn post_starred(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<StarredEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.starred_entries {
        batch.insert(&entry, &())?;
    }
    repo.starred.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn delete_starred(
    Extension(repo): Extension<Arc<Repo>>,
    Json(entries): Json<StarredEntries>,
) -> impl IntoResponse {
    let mut batch = Batch::default();
    for entry in entries.starred_entries {
        batch.remove(&entry)?;
    }
    repo.starred.apply_batch(batch)?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn get_entries(
    Extension(repo): Extension<Arc<Repo>>,
    Query(query): Query<EntriesQuery>,
) -> impl IntoResponse {
    if let Some(true) = query.starred {
        let results = repo
            .starred
            .iter()
            .keys()
            .rev()
            .skip(query.per_page * (query.page - 1))
            .take(query.per_page)
            .map(|res| repo.entries.get(&res?.key()?))
            .filter_map(Result::transpose)
            .collect::<Result<Vec<_>, _>>()?;

        Ok::<_, ServiceEror>((StatusCode::OK, Json(results)))
    } else {
        let res = repo
            .entries
            .iter()
            .values()
            .rev()
            .skip(query.per_page * (query.page - 1))
            .take(query.per_page)
            .collect::<Result<Vec<_>, _>>()?;

        Ok::<_, ServiceEror>((StatusCode::OK, Json(res)))
    }
}

async fn add_subscription(
    Extension(repo): Extension<Arc<Repo>>,
    Json(add_sub): Json<AddSubscription<'_>>,
) -> Result<Response, ServiceEror> {
    let created_at = OffsetDateTime::now_utc();
    let feed = RssClient::default()
        .exec(RssRequest::new(&add_sub.feed_url)?)
        .await?;
    let id = FeedId::generate(&repo.db)?;
    let sub = Subscription::from_feed(id, feed.borrow_feed(), &add_sub.feed_url, created_at);

    repo.subs.insert(&id, &sub)?;
    repo::refresh_feed(repo.clone(), id, feed.borrow_feed()).await?;
    repo.db.flush_async().await?;

    tracing::info!("successfully added a subscription for {}", sub.feed_url);
    Ok((StatusCode::OK, Json(sub)).into_response())
}

async fn delete_subscription(
    Extension(repo): Extension<Arc<Repo>>,
    PathWithExt(feed_id): PathWithExt<FeedId>,
) -> impl IntoResponse {
    repo.subs.remove(&feed_id)?;
    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn refresh_subscriptions(Extension(repo): Extension<Arc<Repo>>) -> impl IntoResponse {
    repo::refresh_all_subsripions(repo.clone()).await?;
    repo.db.flush_async().await?;

    Ok::<_, ServiceEror>(StatusCode::OK)
}

async fn fallback(req: Request<Body>) -> impl IntoResponse {
    tracing::info!("fallback {:?}", req);
    StatusCode::NOT_FOUND
}

#[derive(Debug, Deserialize)]
struct EntriesQuery {
    page: usize,
    per_page: usize,
    starred: Option<bool>,
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
