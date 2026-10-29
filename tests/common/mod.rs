use std::time::Duration;

use axum::{Router, body::Body, http::Request, response::Response};
use newsletter_api::{
    configuration, routes,
    state::{AppState, EmailClient},
    telemetry,
};
use sqlx::PgPool;
use tower::ServiceExt;

pub struct TestApp {
    pub router: Router,
    #[allow(dead_code)]
    pub pool: PgPool,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        telemetry::init_test();
        let configuration = configuration::get().expect("Failed to read configuration.");

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");

        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            Duration::from_millis(200),
        );

        let state = AppState {
            pool: pool.clone(),
            email_client,
        };
        let router = routes::create_route(state);
        Self { router, pool }
    }

    #[allow(dead_code)]
    pub async fn post_subscriptions(&self, body: &str) -> Response {
        let request = Request::builder()
            .uri("/subscriptions")
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .expect("Failed to build request");

        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request")
    }
}
