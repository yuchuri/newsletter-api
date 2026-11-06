use axum::{
    Form,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use hmac::{Hmac, KeyInit, Mac};
use percent_encoding::NON_ALPHANUMERIC;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::PgPool;

use crate::{
    authentication::{self, Credentials},
    error::{AuthError, LoginError},
    state::{DummyPasswordHash, HmacSecret},
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: SecretString,
}

#[tracing::instrument(skip_all, fields(user_id=tracing::field::Empty))]
pub async fn login(
    State(pool): State<PgPool>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    State(hmac_secret): State<HmacSecret>,
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
            let query = format!(
                "error={}",
                percent_encoding::utf8_percent_encode(&e.to_string(), NON_ALPHANUMERIC)
            );
            let hmac_tag = {
                let mut mac =
                    HmacSha256::new_from_slice(hmac_secret.expose_secret().as_bytes()).unwrap();
                mac.update(query.as_bytes());
                hex::encode(mac.finalize().into_bytes())
            };
            let location = format!("/login?{}&tag={}", query, hmac_tag);
            (
                StatusCode::SEE_OTHER,
                [(axum::http::header::LOCATION, location)],
            )
                .into_response()
        }
    }
}
