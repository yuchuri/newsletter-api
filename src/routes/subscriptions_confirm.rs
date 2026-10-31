use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ConfirmRequest {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(pool, request))]
pub async fn confirm(
    State(pool): State<PgPool>,
    Query(request): Query<ConfirmRequest>,
) -> StatusCode {
    let Ok(subscriber_id) = get_subscriber_id_from_token(&pool, &request.subscription_token).await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };

    if let Some(subscriber_id) = subscriber_id {
        if confirm_subscriber(&pool, subscriber_id).await.is_ok() {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    } else {
        StatusCode::UNAUTHORIZED
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscription_id))]
async fn confirm_subscriber(pool: &PgPool, subscription_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        update subscriptions
        set status = 'confirmed'
        where subscriber_id = $1
        "#,
        subscription_id
    )
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {err:?}");
        err
    })?;
    Ok(())
}

#[tracing::instrument(name = "Get_subscriber_id from token", skip(pool, subscription_token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        select subscriber_id
        from subscription_tokens
        where subscription_token = $1
        "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {err:?}");
        err
    })
}
