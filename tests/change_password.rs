mod common;

use sqlx::PgPool;
use uuid::Uuid;

use crate::common::TestApp;

#[sqlx::test]
async fn you_must_be_logged_in_to_see_the_change_password_form(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let response = app.get_change_password().await;

    common::assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn you_must_be_logged_in_to_change_your_password(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let new_password = Uuid::now_v7();

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::now_v7(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    common::assert_is_redirect_to(&response, "/login");
}

#[sqlx::test]
async fn new_password_fields_must_match(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    app.post_login(&serde_json::json!({
        "username": &app.context.test_user.username,
        "password": &app.context.test_user.password
    }))
    .await;

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.context.test_user.username,
            "new_password": Uuid::now_v7(),
            "new_password_check": Uuid::now_v7(),
        }))
        .await;

    common::assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match.</i></p>"
    ));
}

#[sqlx::test]
async fn current_password_must_be_valid(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let new_password = Uuid::now_v7();

    app.post_login(&serde_json::json!({
        "username": &app.context.test_user.username,
        "password": &app.context.test_user.password,
    }))
    .await;

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::now_v7(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    common::assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}

#[sqlx::test]
async fn new_password_must_be_at_least_12_characters_long(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    app.post_login(&serde_json::json!({
        "username": &app.context.test_user.username,
        "password": &app.context.test_user.password
    }))
    .await;

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.context.test_user.password,
            "new_password": "short",
            "new_password_check": "short",
        }))
        .await;

    common::assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(
        html_page
            .contains("<p><i>The new password must be between 12 and 128 characters long.</i></p>")
    );
}

#[sqlx::test]
async fn new_password_must_be_at_most_128_characters_long(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    app.post_login(&serde_json::json!({
        "username": &app.context.test_user.username,
        "password": &app.context.test_user.password
    }))
    .await;

    let new_password = "a".repeat(129);
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.context.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    common::assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(
        html_page
            .contains("<p><i>The new password must be between 12 and 128 characters long.</i></p>")
    );
}

#[sqlx::test]
async fn changing_password_works(pool: PgPool) {
    let app = TestApp::spawn(pool).await;
    let new_password = Uuid::now_v7();

    let response = app
        .post_login(&serde_json::json!({
            "username": &app.context.test_user.username,
            "password": &app.context.test_user.password,
        }))
        .await;
    common::assert_is_redirect_to(&response, "/admin/dashboard");

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.context.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    common::assert_is_redirect_to(&response, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    let response = app.post_logout().await;
    common::assert_is_redirect_to(&response, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    let response = app
        .post_login(&serde_json::json!({
            "username": &app.context.test_user.username,
            "password": &new_password,
        }))
        .await;
    common::assert_is_redirect_to(&response, "/admin/dashboard");
}
