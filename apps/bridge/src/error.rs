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
    #[error("database error")]
    Database(#[from] tokio_postgres::Error),
    #[error("pool error")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("network error: {0}")]
    Reqwest(#[from] reqwest::Error),
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
        };
        let body = Json(ErrorBody {
            error: self.to_string(),
        });
        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
