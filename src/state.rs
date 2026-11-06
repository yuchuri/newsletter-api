use axum::extract::FromRef;
use sqlx::PgPool;

mod application_base_url;
mod dummy_hash;
mod email_client;
mod hmac_secret;

pub use application_base_url::ApplicationBaseUrl;
pub use dummy_hash::DummyPasswordHash;
pub use email_client::EmailClient;
pub use hmac_secret::HmacSecret;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: ApplicationBaseUrl,
    pub dummy_password_hash: DummyPasswordHash,
    pub hmac_secret: HmacSecret,
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

impl FromRef<AppState> for DummyPasswordHash {
    fn from_ref(state: &AppState) -> Self {
        state.dummy_password_hash.clone()
    }
}

impl FromRef<AppState> for HmacSecret {
    fn from_ref(state: &AppState) -> Self {
        state.hmac_secret.clone()
    }
}
