use sqlx::PgPool;

mod email_client;

pub use email_client::EmailClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}
