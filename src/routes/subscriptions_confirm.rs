use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::error::{ConfirmationError, ConfirmationResult};

#[derive(Deserialize)]
pub struct ConfirmQuery {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip_all, err(Debug))]
pub async fn confirm(
    State(pool): State<PgPool>,
    Query(request): Query<ConfirmQuery>,
) -> ConfirmationResult<StatusCode> {
    confirm_subscriber(&pool, &request.subscription_token)
        .await
        .context("Failed to confirm the subscriber.")?
        .then_some(StatusCode::OK)
        .ok_or(ConfirmationError::UnknownTokenError)
}

#[tracing::instrument(name = "Confirm subscriber", skip_all)]
async fn confirm_subscriber(pool: &PgPool, subscription_token: &str) -> sqlx::Result<bool> {
    let confirmed_subscriber = sqlx::query_scalar!(
        r#"
        with confirmed as (
            update subscriptions
            set status = 'confirmed'
            where subscriber_id = (
                select subscriber_id
                from subscription_tokens
                where subscription_token = $1
            )
            and status = 'pending_confirmation'
            returning subscriber_id
        )
        delete from subscription_tokens
        where subscription_token = $1
        and subscriber_id in (select subscriber_id from confirmed)
        returning subscriber_id
        "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await?;
    Ok(confirmed_subscriber.is_some())
}
