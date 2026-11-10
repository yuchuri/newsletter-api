use axum::{
    Form,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
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
    State(pool): State<PgPool>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    Form(request): Form<LoginForm>,
) -> Response {
    let credentials = Credentials::new(request.username, request.password);
    match authentication::validate_credentials(credentials, &pool, &dummy_password_hash).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(user_id));
            (StatusCode::SEE_OTHER, [(header::LOCATION, "/")]).into_response()
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentialsError(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };
            let location = "/login";
            let cookie = Cookie::build(("_flash", e.to_string())).build();

            let jar = CookieJar::new().add(cookie);
            (StatusCode::SEE_OTHER, [(header::LOCATION, location)], jar).into_response()
        }
    }
}
