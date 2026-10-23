use axum::{Form, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SubscribeRequest {
    name: String,
    email: String,
}

pub async fn subscribe(
    State(state): State<AppState>,
    Form(request): Form<SubscribeRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query!(
        r#"
        insert into subscriptions (email, name)
        values ($1, $2)
        "#,
        request.email,
        request.name
    )
    .execute(&state.pool)
    .await
    .map_err(|err| {
        eprintln!("Failed to execute query: {err:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(StatusCode::OK)
}
