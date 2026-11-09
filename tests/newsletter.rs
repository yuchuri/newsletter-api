mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::common::{ConfirmationLinks, TestContext};

async fn create_unconfirmed_subscriber(context: &TestContext) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&context.email_server)
        .await;

    let response = context.post_subscriptions(body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let email_request = context
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    context.get_confirmation_links(&email_request)
}

async fn create_confirmed_subscriber(context: &TestContext) {
    let confirmation_links = create_unconfirmed_subscriber(context).await;
    let response = context.get(confirmation_links.html.as_str()).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_are_not_delivered_to_unconfirmed_subscribers(pool: PgPool) {
    let context = TestContext::new(pool).await;
    create_unconfirmed_subscriber(&context).await;

    Mock::given(matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = context.post_newsletter(newsletter_request_body).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn newsletter_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    let context = TestContext::new(pool).await;
    create_confirmed_subscriber(&context).await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let response = context.post_newsletter(newsletter_request_body).await;
    assert_eq!(response.status(), StatusCode::OK)
}

#[sqlx::test]
async fn newsletters_return_422_for_invalid_data(pool: PgPool) {
    let context = TestContext::new(pool).await;
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
        let response = context.post_newsletter(invalid_body).await;

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
    let context = TestContext::new(pool).await;
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });

    let request = Request::builder()
        .uri("/newsletter")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&newsletter_request_body)
                .expect("Failed to serialize newsletter request."),
        ))
        .expect("Failed to build request.");

    let response = context
        .router
        .clone()
        .oneshot(request)
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}

#[sqlx::test]
async fn invalid_password_is_rejected(pool: PgPool) {
    let context = TestContext::new(pool).await;
    let username = &context.test_user.username;
    // Random password
    let password = Uuid::now_v7().to_string();
    assert_ne!(context.test_user.password, password);

    let request = Request::builder()
        .uri("/newsletter")
        .method("POST")
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!(
                "Basic {}",
                general_purpose::STANDARD.encode(format!("{}:{}", username, password))
            ),
        )
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({
                "title": "Newsletter title",
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }))
            .expect("Failed to serialize newsletter request."),
        ))
        .expect("Failed to build request.");

    let response = context
        .router
        .clone()
        .oneshot(request)
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    )
}
