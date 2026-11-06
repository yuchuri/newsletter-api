use axum::{Form, response::Redirect};
use secrecy::SecretString;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: SecretString,
}

pub async fn login(Form(_request): Form<LoginRequest>) -> Redirect {
    Redirect::to("/")
}
