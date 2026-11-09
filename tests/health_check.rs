mod common;

use axum::{body::Body, http::Request};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::common::TestContext;

#[sqlx::test]
async fn health_check(pool: PgPool) {
    let context = TestContext::new(pool).await;

    let request = Request::builder()
        .uri("/health_check")
        .method("GET")
        .body(Body::empty())
        .expect("Failed to build request");

    let response = context
        .router
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
}
