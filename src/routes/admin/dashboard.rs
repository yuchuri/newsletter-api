use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppResult, session_state::TypedSession};

pub async fn admin_dashboard(
    session: TypedSession,
    State(pool): State<PgPool>,
) -> AppResult<Response> {
    let user_name = if let Some(user_id) = session
        .get_user_id()
        .await
        .context("Failed to retrieve user_id from session")?
    {
        get_username(user_id, &pool).await?
    } else {
        return Ok(Redirect::to("/login").into_response());
    };

    Ok(Html(
        include_str!("admin_dashboard.html")
            .replace("{{USERNAME}}", &html_escape::encode_text(&user_name)),
    )
    .into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> anyhow::Result<String> {
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
