use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::helpers::{ConfirmationLinks, TestApp};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    let response = app.post_form("/subscriptions", body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let email_request = app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    let response = app.get(confirmation_links.html.as_str()).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_are_not_delivered_to_unconfirmed_subscribers(pool: PgPool) {
    let app = TestApp::new(pool).await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = app
        .post_json_with_auth("/newsletter", &newsletter_request_body)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    let app = TestApp::new(pool).await;
    create_confirmed_subscriber(&app).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = app
        .post_json_with_auth("/newsletter", &newsletter_request_body)
        .await;
    assert_eq!(response.status(), StatusCode::OK)
}

#[sqlx::test]
async fn newsletters_return_422_for_invalid_data(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }),
            "missing the title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!"
            }),
            "missing the content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app
            .post_json_with_auth("/newsletter", &invalid_body)
            .await;

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[sqlx::test]
async fn newsletters_missing_authorization_are_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = app.post_json("/newsletter", &newsletter_request_body).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}
