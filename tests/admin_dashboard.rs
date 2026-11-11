mod common;

use sqlx::PgPool;

use crate::common::TestApp;

#[sqlx::test]
async fn you_must_be_logged_in_to_access_admin_dashboard(pool: PgPool) {
    let app = TestApp::spawn(pool).await;

    let response = app.get_admin_dashboard().await;

    common::assert_is_redirect_to(&response, "/login");
}
