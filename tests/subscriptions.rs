mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{Mock, matchers};

use crate::common::TestContext;

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_from_data(pool: PgPool) {
    let context = TestContext::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&context.email_server)
        .await;

    let response = context.post_subscriptions(body).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn subscribe_persists_the_new_subscriber(pool: PgPool) {
    let context = TestContext::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&context.email_server)
        .await;

    context.post_subscriptions(body).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&context.pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[sqlx::test]
async fn subscribe_returns_a_422_when_data_is_missing(pool: PgPool) {
    let context = TestContext::new(pool).await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = context.post_subscriptions(invalid_body).await;

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not fail with 422 Unprocessable Entity when the payload was {error_message}."
        )
    }
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid(pool: PgPool) {
    let context = TestContext::new(pool).await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = context.post_subscriptions(invalid_body).await;

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not return 400 Bad Request when the payload was {error_message}."
        )
    }
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(pool: PgPool) {
    let context = TestContext::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&context.email_server)
        .await;

    context.post_subscriptions(body).await;
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_with_a_link(pool: PgPool) {
    let context = TestContext::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&context.email_server)
        .await;

    context.post_subscriptions(body).await;

    let email_request = &context.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = context.get_confirmation_links(email_request);

    assert_eq!(
        confirmation_links.html, confirmation_links.plain_text,
        "The two links should be identical"
    );
}

#[sqlx::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error(pool: PgPool) {
    let context = TestContext::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!("alter table subscriptions drop column email;")
        .execute(&context.pool)
        .await
        .unwrap();

    let response = context.post_subscriptions(body).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
