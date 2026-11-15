use anyhow::Context;
use askama::Template;
use axum::{Extension, extract::State, response::Html};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{authentication::UserId, error::AppResult};

#[derive(Template)]
#[template(path = "admin_dashboard.html")]
struct AdminDashboardTemplate {
    username: String,
}

pub async fn admin_dashboard(
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
) -> AppResult<Html<String>> {
    let user_id = user_id.into_inner();
    let user_name = get_username(user_id, &pool).await?;

    let template = AdminDashboardTemplate {
        username: user_name,
    };
    Ok(Html(template.render().unwrap()))
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
