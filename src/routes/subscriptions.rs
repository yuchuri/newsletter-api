use std::iter;

use anyhow::Context;
use axum::{Form, extract::State, http::StatusCode};
use rand::{RngExt, distr::Alphanumeric};
use serde::Deserialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    error::{SubscribeError, SubscribeResult},
    state::{ApplicationBaseUrl, EmailClient},
};

#[derive(Deserialize)]
pub struct SubscribeForm {
    name: String,
    email: String,
}

impl TryFrom<SubscribeForm> for NewSubscriber {
    type Error = String;

    fn try_from(value: SubscribeForm) -> Result<Self, Self::Error> {
        let name = SubscriberName::try_from(value.name)?;
        let email = SubscriberEmail::try_from(value.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(name = "Adding a new subscriber", skip_all, err(Debug))]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<ApplicationBaseUrl>,
    Form(request): Form<SubscribeForm>,
) -> SubscribeResult<StatusCode> {
    let new_subscriber = request
        .try_into()
        .map_err(SubscribeError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool.")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert a new subscriber in the database.")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    send_confirmation_email(&email_client, new_subscriber, base_url, &subscription_token)
        .await
        .context("Failed to send a confirmation email.")?;

    Ok(StatusCode::OK)
}

#[tracing::instrument(name = "Send a confirmation email to a new subscriber", skip_all)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: ApplicationBaseUrl,
    subscription_token: &str,
) -> reqwest::Result<()> {
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={subscription_token}");
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription.",
    );
    let plain_body = format!(
        "Welcome to our newsletter!\n\
        Visit {confirmation_link} to confirm your subscription.",
    );

    email_client
        .send_email(&new_subscriber.email, "Welcome", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Saving new subscriber details in the database", skip_all)]
pub async fn insert_subscriber(
    transaction: &mut PgConnection,
    new_subscriber: &NewSubscriber,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar!(
        r#"
        insert into subscriptions (email, name)
        values ($1, $2)
        returning subscriber_id
        "#,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref()
    )
    .fetch_one(transaction)
    .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(transaction, subscription_token)
)]
pub async fn store_token(
    transaction: &mut PgConnection,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        insert into subscription_tokens (subscription_token, subscriber_id)
        values ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    )
    .execute(transaction)
    .await?;
    Ok(())
}

/// Generate a random 25-characters-long acse-sensitive subscription token.
fn generate_subscription_token() -> String {
    let mut rng = rand::rng();
    iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
