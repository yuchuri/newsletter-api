use std::net::SocketAddr;

use reqwest::Client;
use sqlx::PgPool;
use tokio::{net::TcpListener, task::JoinHandle};

use crate::common::test_context::TestContext;

pub struct TestApp {
    pub context: TestContext,
    pub address: SocketAddr,
    pub client: Client,
    pub server: JoinHandle<()>,
}

impl TestApp {
    pub async fn spawn(pool: PgPool) -> Self {
        let context = TestContext::new(pool).await;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let router = context.router.clone();

        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let client = Client::builder().cookie_store(true).build().unwrap();

        Self {
            context,
            address,
            client,
            server,
        }
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.server.abort();
    }
}
