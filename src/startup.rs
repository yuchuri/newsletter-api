use std::io;

use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::state::AppState;

use super::routes;

pub async fn run(listener: TcpListener, pool: PgPool) -> io::Result<()> {
    let state = AppState { pool };
    let app = routes::create_route(state);

    axum::serve(listener, app).await
}
