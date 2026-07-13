use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("too many requests")]
    TooManyRequests,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("upstream service unavailable: {0}")]
    Upstream(String),
    #[error("database error")]
    Database(#[from] tokio_postgres::Error),
    #[error("pool error")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] axum::http::header::InvalidHeaderValue),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Upstream(_) => StatusCode::BAD_GATEWAY,
            AppError::Database(err) => {
                tracing::error!(error = %err, "Internal database error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Pool(err) => {
                tracing::error!(error = %err, "Internal connection pool error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Reqwest(err) => {
                tracing::error!(error = %err, "Internal HTTP client error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Io(err) => {
                tracing::error!(error = %err, "Internal IO error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Json(err) => {
                tracing::error!(error = %err, "Internal JSON error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::InvalidHeaderValue(err) => {
                tracing::error!(error = %err, "Internal header error");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        let client_message = match &self {
            AppError::Unauthorized => "unauthorized".to_string(),
            AppError::NotFound => "not found".to_string(),
            AppError::BadRequest(message) => message.clone(),
            AppError::TooManyRequests => "too many requests".to_string(),
            AppError::Conflict(message) => message.clone(),
            AppError::Upstream(_) => "upstream service temporarily unavailable".to_string(),
            AppError::Database(_)
            | AppError::Pool(_)
            | AppError::Reqwest(_)
            | AppError::Io(_)
            | AppError::Json(_)
            | AppError::InvalidHeaderValue(_) => "internal server error".to_string(),
        };
        let body = Json(ErrorBody {
            error: client_message,
        });
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
