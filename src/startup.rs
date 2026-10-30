use std::io;

use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;

use crate::state::ApplicationBaseUrl;

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
) -> io::Result<()> {
    let state = AppState {
        pool,
        email_client,
        base_url,
    };
    let app = routes::create_route(state);

    axum::serve(listener, app).await
}

pub async fn build(configuration: Settings) -> io::Result<()> {
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

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let listener = TcpListener::bind(address).await?;

    run(listener, configuration_pool, email_client, base_url).await
}
