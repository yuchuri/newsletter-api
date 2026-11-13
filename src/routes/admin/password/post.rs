use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{self, Credentials},
    error::{AppResult, AuthError},
    routes::admin::dashboard::get_username,
    session_state::TypedSession,
    state::DummyPasswordHash,
};

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub current_password: SecretString,
    pub new_password: SecretString,
    pub new_password_check: SecretString,
}

#[tracing::instrument(name = "Change user password", skip_all)]
pub async fn change_password(
    session: TypedSession,
    messages: Messages,
    State(pool): State<PgPool>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    Form(request): Form<ChangePasswordForm>,
) -> AppResult<Response> {
    let Some(user_id) = session.get_user_id().await? else {
        return Ok(Redirect::to("/login").into_response());
    };

    if request.new_password.expose_secret() != request.new_password_check.expose_secret() {
        messages.error("You entered two different new passwords - the field values must match.");
        return Ok(Redirect::to("/admin/password").into_response());
    }

    if !(12..=128).contains(&request.new_password.expose_secret().chars().count()) {
        messages.error("The new password must be between 12 and 128 characters long.");
        return Ok(Redirect::to("/admin/password").into_response());
    }

    let username = get_username(user_id, &pool).await?;
    let credentials = Credentials::new(username, request.current_password);
    if let Err(err) =
        authentication::validate_credentials(credentials, &pool, &dummy_password_hash).await
    {
        return match err {
            AuthError::InvalidCredentials(_) => {
                messages.error("The current password is incorrect.");
                Ok(Redirect::to("/admin/password").into_response())
            }
            AuthError::Unexpected(err) => Err(err.into()),
        };
    }

    authentication::change_password(user_id, request.new_password, &pool).await?;
    messages.info("Your password has been changed.");
    Ok(Redirect::to("/admin/password").into_response())
}
