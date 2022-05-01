use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceEror {
    #[error("db error: {0}")]
    DbError(#[from] sled::Error),
    #[error("transaction failed: {0}")]
    TransactionFailed(#[from] sled::transaction::TransactionError),
    #[error("rss error: {0}")]
    RssError(#[from] rss::client::RssError),
}

impl IntoResponse for ServiceEror {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
