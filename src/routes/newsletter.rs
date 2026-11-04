use anyhow::Context;
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::SubscriberEmail,
    error::{PublishError, PublishResult},
    state::EmailClient,
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

struct Credentials {
    username: String,
    password: SecretString,
}

#[tracing::instrument(name = "Publish a newsletter issue", 
    skip_all, 
    fields(user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    headers: HeaderMap,
    Json(request): Json<PublishNewsletterRequest>,
) -> PublishResult<StatusCode> {
    let credentials = basic_authentication(&headers).map_err(PublishError::AuthError)?;
    let user_id = validate_credentials(credentials, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
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

async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> PublishResult<Uuid> {
    sqlx::query_scalar!(
        r#"
        select user_id
        from users
        where username = $1 and password = $2
        "#,
        credentials.username,
        credentials.password.expose_secret(),
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to validate auth credentials.")
    .map_err(PublishError::UnexpectedError)?;
    .ok_or_else(|| anyhow::anyhow!("Invalid username or password."))
    .map_err(PublishError::AuthError)
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
        .to_string();
    Ok(Credentials {
        username,
        password: SecretString::from(password),
    })
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
