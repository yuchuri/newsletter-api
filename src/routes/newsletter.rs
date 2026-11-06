use anyhow::Context;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    authentication::{self, Credentials},
    domain::SubscriberEmail,
    error::{AuthError, PublishError, PublishResult},
    state::{DummyPasswordHash, EmailClient},
};

#[derive(Deserialize)]
pub struct PublishNewsletterRequest {
    title: String,
    content: Content,
}

#[derive(Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[tracing::instrument(name = "Publish a newsletter issue", 
    fields(user_id=tracing::field::Empty),
    skip_all,
)]
pub async fn publish_newsletter(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(dummy_password_hash): State<DummyPasswordHash>,
    headers: HeaderMap,
    Json(request): Json<PublishNewsletterRequest>,
) -> PublishResult<StatusCode> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    let user_id = authentication::validate_credentials(credentials, &pool, &dummy_password_hash)
        .await
        .map_err(|err| match err {
            AuthError::InvalidCredentialsError(_) => PublishError::AuthError(err.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(err.into()),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(user_id));
    let subscribers = get_confirmed_subscribers(&pool)
        .await
        .context("Failed to fetch confirmed subscribers")?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber,
                        &request.title,
                        &request.content.html,
                        &request.content.text,
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
    Ok(StatusCode::OK)
}

fn basic_authentication(headers: &HeaderMap) -> anyhow::Result<Credentials> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing.")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64_encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authentication scheme was not 'Basic'.")?;
    let decoded_bytes = general_purpose::STANDARD
        .decode(base64_encoded_segment)
        .context("Failed to base64-decode the 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    // APlit into two segments, using ':' as delimitor
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string()
        .into();
    Ok(Credentials::new(username, password))
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
