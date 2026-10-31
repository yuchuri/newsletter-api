use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct ConfirmRequest {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, request))]
pub async fn confirm(
    State(pool): State<PgPool>,
    Query(request): Query<ConfirmRequest>,
) -> StatusCode {
    match confirm_subscriber(&pool, &request.subscription_token).await {
        Ok(true) => StatusCode::OK,
        Ok(false) => StatusCode::UNAUTHORIZED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(name = "Confirm subscriber", skip(pool, subscription_token))]
async fn confirm_subscriber(pool: &PgPool, subscription_token: &str) -> Result<bool, sqlx::Error> {
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
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {err:?}");
        err
    })?;
    Ok(confirmed_subscriber.is_some())
}
