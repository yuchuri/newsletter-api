mod common;

use sqlx::PgPool;

use crate::common::TestApp;

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&login_body).await;

    common::assert_is_redirect_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[sqlx::test]
async fn redirect_to_admin_dashboard_after_login_success(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let login_body = serde_json::json!({
        "username": &app.context.test_user.username,
        "password": &app.context.test_user.password
    });
    let response = app.post_login(&login_body).await;

    common::assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", &app.context.test_user.username)));
}
