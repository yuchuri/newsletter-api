use std::io;

use newsletter_api::{configuration, startup, telemetry};

#[tokio::main]
async fn main() -> io::Result<()> {
    telemetry::init();
    let configuration = configuration::get().expect("Failed to read configuration.");

    startup::build(configuration).await
}
