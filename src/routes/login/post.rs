use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{self, Credentials},
    error::{AuthError, LoginError},
    state::DummyPasswordHash,
};

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: SecretString,
}

#[tracing::instrument(skip_all, fields(user_id=tracing::field::Empty))]
pub async fn login(
    messages: Messages,
    State(pool): State<PgPool>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    Form(request): Form<LoginForm>,
) -> Redirect {
    let credentials = Credentials::new(request.username, request.password);
    match authentication::validate_credentials(credentials, &pool, &dummy_password_hash).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(user_id));
            Redirect::to("/")
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentialsError(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            let location = "/login";
            messages.error(e.to_string());
            Redirect::to(location)
        }
    }
}
