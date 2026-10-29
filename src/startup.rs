use std::io;

use sqlx::PgPool;
use tokio::net::TcpListener;

use super::state::{AppState, EmailClient};

use super::routes;

pub async fn run(listener: TcpListener, pool: PgPool, email_client: EmailClient) -> io::Result<()> {
    let state = AppState { pool, email_client };
    let app = routes::create_route(state);

    axum::serve(listener, app).await
}
