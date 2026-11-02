use std::fmt::Debug;

use axum::response::IntoResponse;

pub type ConfirmationResult<T> = Result<T, ConfirmationError>;
#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("There is no subscriber associated with the provided token.")]
    UnknownTokenError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> axum::response::Response {
        use ConfirmationError::*;

        match self {
            UnknownTokenError => axum::http::StatusCode::UNAUTHORIZED,
            UnexpectedError(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response()
    }
}
