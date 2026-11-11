use std::net::SocketAddr;

use reqwest::{Client, Response, StatusCode, redirect::Policy};
use serde::Serialize;
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

        let client = Client::builder()
            .cookie_store(true)
            .redirect(Policy::none())
            .build()
            .unwrap();

        Self {
            context,
            address,
            client,
            server,
        }
    }

    pub async fn post_login(&self, body: &impl Serialize) -> Response {
        self.client
            .post(format!("http://{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.client
            .get(format!("http://{}/login", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> Response {
        self.client
            .get(format!("http://{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.client
            .get(format!("http://{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }
}

pub fn assert_is_redirect_to(response: &Response, location: &str) {
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
        location
    );
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.server.abort();
    }
}
