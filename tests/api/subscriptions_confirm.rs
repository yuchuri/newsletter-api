use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::helpers::TestApp;

#[sqlx::test]
async fn confirmations_without_token_are_rejected_with_a_400(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app.get("/subscriptions/confirm").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn the_link_returned_by_subscribe_return_a_200_if_called(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post("/subscriptions", body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    let response = app.get(confirmation_links.html.as_str()).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post("/subscriptions", body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    app.get(confirmation_links.html.as_str()).await;

    let saved = sqlx::query!("select email, name, status from subscriptions",)
        .fetch_one(&app.pool)
        .await
        .expect("Failed to fetch subscriptions.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    assert_eq!(saved.status, "confirmed");
}
