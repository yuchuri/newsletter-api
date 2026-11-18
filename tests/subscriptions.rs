mod common;

use axum::http::StatusCode;
use sqlx::PgPool;

use crate::common::TestApp;

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_from_data(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let body = serde_json::json!({
        "name": "le guin",
        "email": "ursula_le_guin@gmail.com"
    });

    common::when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(&body).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn subscribe_persists_the_new_subscriber(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let body = serde_json::json!({
        "name": "le guin",
        "email": "ursula_le_guin@gmail.com"
    });

    common::when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(&body).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "pending_confirmation");
}

#[sqlx::test]
async fn subscribe_returns_a_422_when_data_is_missing(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let test_cases = vec![
        (
            serde_json::json!({ "name": "le guin" }),
            "missing the email",
        ),
        (
            serde_json::json!({ "email": "ursula_le_guin@gmail.com" }),
            "missing the name",
        ),
        (serde_json::json!({}), "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(&invalid_body).await;

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not fail with 422 Unprocessable Entity when the payload was {error_message}."
        )
    }
}

#[sqlx::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let test_cases = vec![
        (
            serde_json::json!({ "name": "", "email": "ursula_le_guin@gmail.com" }),
            "empty name",
        ),
        (
            serde_json::json!({ "name": "Ursula", "email": "" }),
            "empty email",
        ),
        (
            serde_json::json!({ "name": "Ursula", "email": "definitely-not-an-email" }),
            "invalid email",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(&invalid_body).await;

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not return 400 Bad Request when the payload was {error_message}."
        )
    }
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let body = serde_json::json!({
        "name": "le guin",
        "email": "ursula_le_guin@gmail.com"
    });

    common::when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(&body).await;
}

#[sqlx::test]
async fn subscribe_sends_a_confirmation_email_with_a_link(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let body = serde_json::json!({
        "name": "le guin",
        "email": "ursula_le_guin@gmail.com"
    });

    common::when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(&body).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    assert_eq!(
        confirmation_links.html, confirmation_links.plain_text,
        "The two links should be identical"
    );
}

#[sqlx::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let body = serde_json::json!({
        "name": "le guin",
        "email": "ursula_le_guin@gmail.com"
    });

    sqlx::query!("alter table subscriptions drop column email;")
        .execute(&app.pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(&body).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
