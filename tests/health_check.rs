mod common;

use axum::{body::Body, http::Request};
use common::TestApp;
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test]
async fn health_check(pool: PgPool) {
    let app = TestApp::new(pool);

    let request = Request::builder()
        .uri("/health_check")
        .method("GET")
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
}
