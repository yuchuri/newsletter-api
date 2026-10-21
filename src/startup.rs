use std::io;

use tokio::net::TcpListener;

use super::routes;

pub async fn run() -> io::Result<()> {
    let app = routes::create_route();

    let listener = TcpListener::bind("127.0.0.1:8000").await?;

    axum::serve(listener, app).await
}
