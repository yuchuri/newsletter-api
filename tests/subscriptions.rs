mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use common::TestApp;
use sqlx::PgPool;
use tower::ServiceExt;

impl TestApp {
    async fn post_subscriptions(&self, body: &str) -> Response {
        let request = Request::builder()
            .uri("/subscriptions")
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .expect("Failed to build request");

        self.app
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request")
    }
}

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_from_data(pool: PgPool) {
    let app = TestApp::new(pool);

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body).await;

    assert_eq!(response.status(), StatusCode::OK);

    let saved = sqlx::query!("select email, name from subscriptions")
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[sqlx::test]
async fn subscribe_returns_a_422_when_data_is_missing(pool: PgPool) {
    let app = TestApp::new(pool);

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body).await;

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not fail with 422 Unprocessable Entity when the payload was {error_message}."
        )
    }
}
