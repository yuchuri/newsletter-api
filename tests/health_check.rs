mod common;

use sqlx::PgPool;

use crate::common::TestApp;

#[sqlx::test]
async fn health_check(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let response = app
        .api_client
        .get(format!("http://{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(response.content_length(), Some(0));
}
