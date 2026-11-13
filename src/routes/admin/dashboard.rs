use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppResult, session_state::TypedSession};

#[tracing::instrument(name = "Admin dashboard", skip_all, err(Debug))]
pub async fn admin_dashboard(
    session: TypedSession,
    State(pool): State<PgPool>,
) -> AppResult<Response> {
    let Some(user_id) = session.get_user_id().await? else {
        return Ok(Redirect::to("/login").into_response());
    };
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
