use anyhow::Context;
use axum::{Form, extract::State, response::Redirect};
use axum_messages::Messages;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, error::AppResult, state::EmailClient};

#[derive(Deserialize)]
pub struct PublishNewsletterForm {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(name = "Publish a newsletter issue", skip_all)]
pub async fn publish_newsletter(
    messages: Messages,
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    Form(request): Form<PublishNewsletterForm>,
) -> AppResult<Redirect> {
    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber,
                        &request.title,
                        &request.text_content,
                        &request.html_content,
                    )
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
    Ok(Redirect::to("/admin/newsletter"))
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
