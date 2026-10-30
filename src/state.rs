use axum::extract::FromRef;
use sqlx::PgPool;

mod application_base_url;
mod email_client;

pub use application_base_url::ApplicationBaseUrl;
pub use email_client::EmailClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: ApplicationBaseUrl,
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

impl FromRef<AppState> for ApplicationBaseUrl {
    fn from_ref(state: &AppState) -> Self {
        state.base_url.clone()
    }
}
