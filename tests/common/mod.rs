use newsletter_api::{routes, state::AppState, telemetry};
use sqlx::PgPool;

pub struct TestApp {
    pub app: axum::Router,
    #[allow(dead_code)]
    pub pool: PgPool,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        telemetry::init_test();
        let state = AppState { pool: pool.clone() };
        let app = routes::create_route(state);
        Self { app, pool }
    }
}
