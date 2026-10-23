use std::io;

use newsletter_api::{configuration, startup, telemetry};
use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> io::Result<()> {
    telemetry::init();
    let configuration = configuration::get().expect("Failed to read configuration.");
    let pool = PgPool::connect(configuration.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");

    let listener =
        TcpListener::bind(format!("127.0.0.1:{}", configuration.application_port)).await?;

    startup::run(listener, pool).await
}
