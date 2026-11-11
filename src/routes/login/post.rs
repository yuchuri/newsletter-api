use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use secrecy::SecretString;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{self, Credentials},
    error::{AuthError, LoginError, LoginResult},
    session_state::TypedSession,
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
    session: TypedSession,
    State(pool): State<PgPool>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    Form(request): Form<LoginForm>,
) -> LoginResult<Redirect> {
    let credentials = Credentials::new(request.username, request.password);
    let user_id = authentication::validate_credentials(credentials, &pool, &dummy_password_hash)
        .await
        .map_err(|err| {
            let err = match err {
                AuthError::InvalidCredentialsError(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
            };
            messages.clone().error(err.to_string());
            err
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(user_id));
    session.cycle_id().await.map_err(|err| {
        let err = LoginError::UnexpectedError(err.into());
        messages.clone().error(err.to_string());
        err
    })?;
    session.insert_user_id(user_id).await.map_err(|err| {
        let err = LoginError::UnexpectedError(err.into());
        messages.clone().error(err.to_string());
        err
    })?;
    Ok(Redirect::to("/admin/dashboard"))
}
