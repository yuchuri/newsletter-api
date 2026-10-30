use std::time::Duration;

use axum::{Router, body::Body, http::Request, response::Response};
use linkify::{LinkFinder, LinkKind};
use newsletter_api::{
    configuration, routes,
    state::{AppState, ApplicationBaseUrl, EmailClient},
    telemetry,
};
use reqwest::Url;
use sqlx::PgPool;
use tower::ServiceExt;
use wiremock::MockServer;

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    pub email_server: MockServer,
}

pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

impl TestApp {
    pub async fn new(pool: PgPool) -> Self {
        telemetry::init_test();

        let email_server = MockServer::start().await;

        let configuration = configuration::get().expect("Failed to read configuration.");

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");

        let email_client = EmailClient::new(
            email_server.uri(),
            sender_email,
            configuration.email_client.authorization_token,
            Duration::from_millis(200),
        );

        let base_url = ApplicationBaseUrl::new(configuration.application.base_url);

        let state = AppState {
            pool: pool.clone(),
            email_client,
            base_url,
        };
        let router = routes::create_route(state);
        Self {
            router,
            pool,
            email_server,
        }
    }

    pub async fn get(&self, uri: &str) -> Response {
        let request = Request::builder()
            .uri(uri)
            .method("GET")
            .body(Body::empty())
            .expect("Failed to build request");

        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request")
    }

    pub async fn post(&self, uri: &str, body: &str) -> Response {
        let request = Request::builder()
            .uri(uri)
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .expect("Failed to build request");

        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value =
            serde_json::from_slice(&email_request.body).expect("Failed to parse request body");

        let get_link = |s: &str| {
            let links: Vec<_> = LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }
}
