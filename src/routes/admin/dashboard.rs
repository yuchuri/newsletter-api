use anyhow::Context;
use axum::{
    Extension,
    extract::State,
    response::{Html, IntoResponse, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{authentication::UserId, error::AppResult};

pub async fn admin_dashboard(
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
) -> AppResult<Response> {
    let user_id = user_id.into_inner();
    let user_name = get_username(user_id, &pool).await?;

    Ok(Html(
        include_str!("admin_dashboard.html")
            .replace("{{USERNAME}}", &html_escape::encode_text(&user_name)),
    )
    .into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> anyhow::Result<String> {
    sqlx::query_scalar!(
        r#"
        select username
        from users
        where user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to fetch username from database")
}
