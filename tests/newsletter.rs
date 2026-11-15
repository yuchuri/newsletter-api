mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::common::{ConfirmationLinks, TestApp, TestContext};

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
    let app = TestApp::spawn(pool).await;
    create_unconfirmed_subscriber(&app.context).await;
    app.login().await;

    Mock::given(matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    common::assert_is_redirect_to(&response, "/admin/newsletter");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[sqlx::test]
async fn newsletter_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    create_confirmed_subscriber(&app.context).await;
    app.login().await;

    Mock::given(matchers::path("/email"))
        .and(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    common::assert_is_redirect_to(&response, "/admin/newsletter");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_publish_newsletter_form(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let response = app.get_publish_newsletter().await;

    common::assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn you_must_be_logged_in_to_publish_a_newsletter(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    common::assert_is_redirect_to(&response, "/login");
}
