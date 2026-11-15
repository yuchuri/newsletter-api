use anyhow::Context;
use axum::{
    Extension, Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::UserId,
    domain::SubscriberEmail,
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyKey},
    state::EmailClient,
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
    State(email_client): State<EmailClient>,
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
    if let Some(saved_response) =
        idempotency::get_saved_response(&pool, &idempotency_key, user_id).await?
    {
        messages.info("The newsletter issue has been published!");
        return Ok(saved_response);
    }
    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber, &title, &text_content, &html_content)
                    .await
                    .with_context(|| format!("Failed to send newsletter issue to {subscriber}"))?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid"
                );
            }
        }
    }
    messages.info("The newsletter issue has been published!");
    let response = Redirect::to("/admin/newsletter").into_response();
    let response = idempotency::save_response(&pool, &idempotency_key, user_id, response).await?;
    Ok(response)
}

#[tracing::instrument(name = "Get confirmed subscribers", skip_all)]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> anyhow::Result<Vec<anyhow::Result<SubscriberEmail>>> {
    let subscribers = sqlx::query_scalar!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|email| match SubscriberEmail::try_from(email) {
        Ok(email) => Ok(email),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();

    Ok(subscribers)
}
