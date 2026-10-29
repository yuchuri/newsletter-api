use std::{io, time::Duration};

use newsletter_api::{configuration, startup, state::EmailClient, telemetry};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    telemetry::init();
    let configuration = configuration::get().expect("Failed to read configuration.");
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
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

    let listener = TcpListener::bind(format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    ))
    .await?;

    startup::run(listener, pool, email_client).await
}
