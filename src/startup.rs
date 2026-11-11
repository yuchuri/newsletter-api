use base64::{Engine, engine::general_purpose};
use secrecy::ExposeSecret;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tower_sessions::{MemoryStore, SessionManagerLayer, cookie::Key};

use crate::state::{ApplicationBaseUrl, DummyPasswordHash};

use super::{
    configuration::Settings,
    routes,
    state::{AppState, EmailClient},
};

pub async fn run(
    listener: TcpListener,
    pool: PgPool,
    email_client: EmailClient,
    base_url: ApplicationBaseUrl,
    dummy_password_hash: DummyPasswordHash,
    session_key: Key,
) -> anyhow::Result<()> {
    let state = AppState {
        pool,
        email_client,
        base_url,
        dummy_password_hash,
    };
    let app = routes::create_route(state).layer(
        SessionManagerLayer::new(MemoryStore::default())
            .with_secure(false)
            .with_signed(session_key),
    );
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn build(configuration: Settings) -> anyhow::Result<()> {
    let configuration_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.connection_options());

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let timeout = configuration.email_client.timeout();

    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );

    let base_url = ApplicationBaseUrl::new(configuration.application.base_url);
    let dummy_password_hash = DummyPasswordHash::generate();
    let session_key = Key::try_from(
        general_purpose::STANDARD
            .decode(configuration.application.hmac_secret.expose_secret())?
            .as_slice(),
    )?;

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let listener = TcpListener::bind(address).await?;

    run(
        listener,
        configuration_pool,
        email_client,
        base_url,
        dummy_password_hash,
        session_key,
    )
    .await
}
