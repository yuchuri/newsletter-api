use std::io;

use newsletter_api::startup;

#[tokio::main]
async fn main() -> io::Result<()> {
    startup::run().await
}
