use anyhow::Context;
use axum::{
    Extension, Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    authentication::UserId,
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyKey, NextAction},
};

#[derive(Deserialize)]
pub struct PublishNewsletterForm {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[tracing::instrument(name = "Publish a newsletter issue", skip_all)]
pub async fn publish_newsletter(
    messages: Messages,
    Extension(user_id): Extension<UserId>,
    State(pool): State<PgPool>,
    Form(request): Form<PublishNewsletterForm>,
) -> AppResult<Response> {
    let PublishNewsletterForm {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = request;
    let idempotency_key =
        IdempotencyKey::try_from(idempotency_key).map_err(AppError::BadRequest)?;
    let user_id = user_id.into_inner();

    let mut transaction =
        match idempotency::try_processing(&pool, &idempotency_key, user_id).await? {
            NextAction::StartProcessing(transaction) => transaction,
            NextAction::ReturnSavedResponse(saved_response) => {
                success_message(messages);
                return Ok(saved_response);
            }
        };
    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newsletter issue details.")?;
    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks.")?;
    let response = Redirect::to("/admin/newsletter").into_response();
    let response =
        idempotency::save_response(transaction, &idempotency_key, user_id, response).await?;
    success_message(messages);
    Ok(response)
}

fn success_message(messages: Messages) {
    messages.info("The newsletter issue has been published!");
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    connection: &mut PgConnection,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar!(
        r#"
        insert into newsletter_issues (
            title,
            text_content,
            html_content
        )
        values ($1, $2, $3)
        returning newsletter_issue_id
        "#,
        title,
        text_content,
        html_content
    )
    .fetch_one(connection)
    .await
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    connection: &mut PgConnection,
    newsletter_issue_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        insert into issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        select $1, email
        from subscriptions
        where status = 'confirmed'
        "#,
        newsletter_issue_id
    )
    .execute(connection)
    .await?;
    Ok(())
}
