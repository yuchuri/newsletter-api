use std::iter;

use axum::{Form, extract::State, http::StatusCode};
use rand::{RngExt, distr::Alphanumeric};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    state::{ApplicationBaseUrl, EmailClient},
};

#[derive(Deserialize)]
pub struct SubscribeRequest {
    name: String,
    email: String,
}

impl TryFrom<SubscribeRequest> for NewSubscriber {
    type Error = String;

    fn try_from(value: SubscribeRequest) -> Result<Self, Self::Error> {
        let name = SubscriberName::try_from(value.name)?;
        let email = SubscriberEmail::try_from(value.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(pool, email_client, request, base_url),
    fields(
        request_id = %Uuid::now_v7(),
        user_email = %request.email,
        user_name = %request.name
    )
)]
pub async fn subscribe(
    State(pool): State<PgPool>,
    State(email_client): State<EmailClient>,
    State(base_url): State<ApplicationBaseUrl>,
    Form(request): Form<SubscribeRequest>,
) -> StatusCode {
    let new_subscriber = match request.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let Ok(subscriber_id) = insert_subscriber(&pool, &new_subscriber).await else {
        return StatusCode::INTERNAL_SERVER_ERROR;
    };
    let subscription_token = generate_subscription_token();
    if store_subscription_token(&pool, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if send_confirmation_email(&email_client, new_subscriber, base_url, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        insert into subscriptions (email, name)
        values ($1, $2)
        returning subscriber_id
        "#,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref()
    )
    .fetch_one(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {err:?}");
        err
    })
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url)
)]
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
        .send_email(new_subscriber.email, "Welcome", &html_body, &plain_body)
        .await
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, pool)
)]
pub async fn store_subscription_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into subscription_tokens (subscription_token, subscriber_id)
        values ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    )
    .execute(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to execute query: {err:?}");
        err
    })?;
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
