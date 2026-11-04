use std::fmt::Debug;

use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

pub type PublishResult<T> = Result<T, PublishError>;

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> Response {
        use PublishError::*;

        match self {
            AuthError(_) => (
                StatusCode::UNAUTHORIZED,
                [(
                    header::WWW_AUTHENTICATE,
                    HeaderValue::from_static(r#"Basic realm="publish""#),
                )],
            )
                .into_response(),
            UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
