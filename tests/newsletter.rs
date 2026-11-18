mod common;

use sqlx::PgPool;
use uuid::Uuid;
use wiremock::{Mock, ResponseTemplate, matchers};

use crate::common::TestApp;

#[sqlx::test]
async fn newsletter_are_not_delivered_to_unconfirmed_subscribers(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    app.create_unconfirmed_subscriber().await;
    app.login().await;

    Mock::given(matchers::any())
        .respond_with(wiremock::ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
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
    app.dispatch_all_pending_emails().await;
}

#[sqlx::test]
async fn newsletter_are_delivered_to_confirmed_subscribers(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    app.create_confirmed_subscriber().await;
    app.login().await;

    common::when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
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
    app.dispatch_all_pending_emails().await;
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
    app.create_confirmed_subscriber().await;

    common::when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
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
    app.dispatch_all_pending_emails().await;
}

#[sqlx::test]
async fn concurrent_form_submission_is_handled_gracefully(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    app.login().await;
    app.create_confirmed_subscriber().await;

    common::when_sending_an_email()
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
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
    app.dispatch_all_pending_emails().await;
}
