use axum::{body::Body, http::Request};
use newsletter_api::routes;
use tower::ServiceExt;

#[tokio::test]
async fn health_check() {
    let app = routes::create_route();

    let request = Request::builder()
        .uri("/health_check")
        .method("GET")
        .body(Body::empty())
        .expect("Failed to build request");

    let response = app
        .oneshot(request)
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success())
}
