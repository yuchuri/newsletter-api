use axum::extract::FromRef;
use sqlx::PgPool;

mod email_client;

pub use email_client::EmailClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub email_client: EmailClient,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for EmailClient {
    fn from_ref(state: &AppState) -> Self {
        state.email_client.clone()
    }
}
