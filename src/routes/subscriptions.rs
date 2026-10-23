use axum::{Form, extract::State, http::StatusCode};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SubscribeRequest {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(state, request),
    fields(
        request_id = %Uuid::now_v7(),
        user_email = %request.email,
        user_name = %request.name
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(request): Form<SubscribeRequest>,
) -> StatusCode {
    match insert_subscriber(&state.pool, &request).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(
    pool: &sqlx::PgPool,
    form: &SubscribeRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into subscriptions (email, name)
        values ($1, $2)
        "#,
        form.email,
        form.name
    )
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {err:?}");
        err
    })?;
    Ok(())
}
