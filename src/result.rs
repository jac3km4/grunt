use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rsst::client::RssError;
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceEror {
    #[error("db error: {0}")]
    DbError(#[from] sled_bincode::Error),
    #[error("transaction failed: {0}")]
    TransactionFailed(#[from] sled_bincode::TransactionError),
    #[error(
        "{0} ({})",
        if matches!(.0, RssError::XmlDecode(_)) { "possibly RSS 1.0" } else { "RSS lookup failed" })
    ]
    RssError(#[from] RssError),
}

pub type Result<A, E = ServiceEror> = std::result::Result<A, E>;

impl From<sled_bincode::SledError> for ServiceEror {
    fn from(err: sled_bincode::SledError) -> Self {
        Self::DbError(sled_bincode::Error::SledError(err))
    }
}

impl IntoResponse for ServiceEror {
    fn into_response(self) -> Response {
        let body = json!({"message": self.to_string()});
        (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
    }
}
