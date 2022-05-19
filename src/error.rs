use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceEror {
    #[error("db error: {0}")]
    DbError(#[from] sled_bincode::Error),
    #[error("transaction failed: {0}")]
    TransactionFailed(#[from] sled_bincode::TransactionError),
    #[error("rss client error: {0}")]
    RssError(#[from] rsst::client::RssError),
}

impl From<sled_bincode::SledError> for ServiceEror {
    fn from(err: sled_bincode::SledError) -> Self {
        Self::DbError(sled_bincode::Error::SledError(err))
    }
}

impl IntoResponse for ServiceEror {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
