mod common;

use axum::http::StatusCode;
use fake::{
    Fake,
    faker::{internet::en::SafeEmail, name::en::Name},
};
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::{Mock, MockBuilder, ResponseTemplate, matchers};

use crate::common::{ConfirmationLinks, TestApp, TestContext};

fn when_sending_an_email() -> MockBuilder {
    Mock::given(matchers::path("/email")).and(matchers::method("POST"))
}

async fn create_unconfirmed_subscriber(context: &TestContext) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();

    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();

    let _mock_guard = when_sending_an_email()
        .respond_with(wiremock::ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&context.email_server)
        .await;

    let response = context.post_subscriptions(&body).await;
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
        "idempotency_key": Uuid::now_v7().to_string()
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

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": Uuid::now_v7().to_string()
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
        "idempotency_key": Uuid::now_v7().to_string()
    });

    let response = app.post_publish_newsletter(&newsletter_request_body).await;

    common::assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn newsletter_creation_is_idempotent(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    app.login().await;
    create_confirmed_subscriber(&app.context).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": &Uuid::now_v7().to_string()
    });
    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    common::assert_is_redirect_to(&response, "/admin/newsletter");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    common::assert_is_redirect_to(&response, "/admin/newsletter");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
}

#[sqlx::test]
async fn concurrent_form_submission_is_handled_gracefully(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    app.login().await;
    create_confirmed_subscriber(&app.context).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.context.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": Uuid::now_v7().to_string()
    });

    let response1 = app.post_publish_newsletter(&newsletter_request_body);
    let response2 = app.post_publish_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
}

#[sqlx::test]
async fn transient_errors_do_not_cause_duplicate_deliveries_on_retries(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    app.login().await;
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": Uuid::now_v7().to_string()
    });

    create_confirmed_subscriber(&app.context).await;
    create_confirmed_subscriber(&app.context).await;

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.context.email_server)
        .await;
    when_sending_an_email()
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.context.email_server)
        .await;

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .named("Delivery retry")
        .mount(&app.context.email_server)
        .await;

    let response = app.post_publish_newsletter(&newsletter_request_body).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}
