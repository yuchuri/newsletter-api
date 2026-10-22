use axum::{Form, http::StatusCode};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SubscribeRequest {
    name: String,
    email: String,
}

pub async fn subscribe(Form(_request): Form<SubscribeRequest>) -> StatusCode {
    StatusCode::OK
}
