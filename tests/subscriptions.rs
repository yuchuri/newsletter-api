use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use newsletter_api::routes;
use tower::ServiceExt;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_from_data() {
    let app = routes::create_route();

    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let resquest = Request::builder()
        .uri("/subscriptions")
        .method("POST")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .expect("Failed to build request");

    let response = app
        .oneshot(resquest)
        .await
        .expect("Failed to execute resquest");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    let app = routes::create_route();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let resquest = Request::builder()
            .uri("/subscriptions")
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(invalid_body))
            .expect("Failed to build request");

        let response = app
            .clone()
            .oneshot(resquest)
            .await
            .expect("Failed to execute resquest");

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not fail with 422 Unprocessable Entity when the payload was {error_message}."
        )
    }
}
