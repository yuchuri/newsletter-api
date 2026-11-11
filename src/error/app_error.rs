use std::fmt::Debug;

use axum::{http::StatusCode, response::IntoResponse};

pub type AppResult<T> = Result<T, AppError>;

#[derive(thiserror::Error)]
#[error("Internal server error")]
pub struct AppError(#[from] anyhow::Error);

impl Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
