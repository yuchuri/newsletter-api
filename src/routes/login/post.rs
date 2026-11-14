use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{self, Credentials},
    error::{AppError, AppResult, AuthError},
    session_state::TypedSession,
    state::DummyPasswordHash,
};

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: SecretString,
}

#[tracing::instrument(name = "User login", skip_all, fields(user_id=tracing::field::Empty))]
pub async fn login(
    messages: Messages,
    session: TypedSession,
    State(pool): State<PgPool>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    Form(request): Form<LoginForm>,
) -> AppResult<Redirect> {
    let credentials = Credentials::new(request.username, request.password);
    let user_id = authentication::validate_credentials(credentials, &pool, &dummy_password_hash)
        .await
        .map_err(|err| match err {
            AuthError::Unexpected(_) => AppError::Unexpected(err.into()),
            AuthError::InvalidCredentials(_) => {
                let err = AppError::InvalidAuthentication(err.into());
                messages.error(err.to_string());
                err.redirect("/login")
            }
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));
    session.cycle_id().await?;
    session.insert_user_id(user_id).await?;
    Ok(Redirect::to("/admin/dashboard"))
}
