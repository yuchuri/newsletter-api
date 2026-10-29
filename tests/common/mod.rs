use std::time::Duration;

use newsletter_api::{
    configuration, routes,
    state::{AppState, EmailClient},
    telemetry,
};
use sqlx::PgPool;

pub struct TestApp {
    pub app: axum::Router,
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
        let app = routes::create_route(state);
        Self { app, pool }
    }
}
