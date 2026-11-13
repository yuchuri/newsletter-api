use axum::{
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};

pub type AuthResult<T> = Result<T, AuthError>;
pub type AppResult<T> = Result<T, AppError>;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),

    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,

    #[error("Authentication failed")]
    BasicAuth(#[source] anyhow::Error),

    #[error("{0}")]
    Redirect(#[source] Box<AppError>, Redirect),

    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl AppError {
    pub fn redirect(self, location: &str) -> Self {
        AppError::Redirect(Box::new(self), Redirect::to(location))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use AppError::*;

        match self {
            Validation(_) => {
                tracing::warn!("{self:?}");
                StatusCode::BAD_REQUEST.into_response()
            }

            UnknownToken => {
                tracing::warn!("{self:?}");
                StatusCode::UNAUTHORIZED.into_response()
            }

            BasicAuth(_) => {
                tracing::warn!("{self:?}");
                (
                    StatusCode::UNAUTHORIZED,
                    [(
                        header::WWW_AUTHENTICATE,
                        HeaderValue::from_static(r#"Basic realm="publish""#),
                    )],
                )
                    .into_response()
            }

            Redirect(err, redirect) => {
                tracing::warn!("{err:?}");
                redirect.into_response()
            }

            Unexpected(_) => {
                tracing::error!("{self:?}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
