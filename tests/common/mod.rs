#![expect(dead_code)]
use std::{env, io, sync::LazyLock};

use argon2::{Argon2, PasswordHasher};
use fake::{
    Fake,
    faker::{internet::en::SafeEmail, name::en::Name},
};
use linkify::{LinkFinder, LinkKind};
use newsletter_api::{
    configuration,
    issue_delivery_worker::{self, ExecutionOutcome},
    startup::Application,
    state::EmailClient,
};
use reqwest::{Client, Response, StatusCode, Url, redirect::Policy};
use serde::Serialize;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tracing_subscriber::{EnvFilter, fmt::writer::BoxMakeWriter};
use uuid::Uuid;
use wiremock::{Mock, MockBuilder, MockServer, ResponseTemplate, matchers};

pub fn when_sending_an_email() -> MockBuilder {
    Mock::given(matchers::path("/email")).and(matchers::method("POST"))
}

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let writer = if env::var("TEST_LOG").is_ok() {
        BoxMakeWriter::new(io::stdout)
    } else {
        BoxMakeWriter::new(io::sink)
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .pretty()
        .with_writer(writer)
        .init();
});

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub pool: PgPool,
    pub test_user: TestUser,
    pub api_server: JoinHandle<()>,
    pub api_client: Client,
    pub email_server: MockServer,
    pub email_client: EmailClient,
}

pub struct TestUser {
    pub username: String,
    pub password: String,
}

pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

impl TestApp {
    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                issue_delivery_worker::try_execute_task(&self.pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }

    pub async fn spawn(pool: PgPool) -> Self {
        LazyLock::force(&TRACING);
        let mut configuration = configuration::get().expect("Failed to read configuration.");

        let options = pool.connect_options();
        if let Some(db_name) = options.get_database() {
            configuration.database.database_name = db_name.to_owned();
        }
        configuration.database.host = options.get_host().to_owned();
        configuration.database.port = options.get_port().to_owned();
        configuration.database.username = options.get_username().to_owned();

        configuration.application.port = 0;
        let email_server = MockServer::start().await;
        configuration.email_client.base_url = email_server.uri();
        let email_client = configuration.email_client.clone().client();

        let application = Application::build(configuration)
            .await
            .expect("Failed to build application.");
        let address = format!("127.0.0.1:{}", application.port());
        let port = application.port();

        let api_server = tokio::spawn(async move { application.run().await.unwrap() });

        let api_client = Client::builder()
            .cookie_store(true)
            .redirect(Policy::none())
            .build()
            .unwrap();

        let test_user = TestUser::generate(&pool).await;

        Self {
            address,
            port,
            pool,
            test_user,
            api_client,
            api_server,
            email_client,
            email_server,
        }
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
            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }

    pub async fn login(&self) {
        self.post_login(&serde_json::json!({
            "username": self.test_user.username,
            "password": self.test_user.password,
        }))
        .await;
    }

    pub async fn post_subscriptions(&self, body: &impl Serialize) -> Response {
        self.api_client
            .post(format!("http://{}/subscriptions", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_login(&self, body: &impl Serialize) -> Response {
        self.api_client
            .post(format!("http://{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(format!("http://{}/login", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> Response {
        self.api_client
            .get(format!("http://{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> Response {
        self.api_client
            .get(format!("http://{}/admin/password", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password(&self, body: &impl Serialize) -> Response {
        self.api_client
            .post(format!("http://{}/admin/password", self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_logout(&self) -> Response {
        self.api_client
            .post(format!("http://{}/admin/logout", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_publish_newsletter(&self, body: &serde_json::Value) -> Response {
        self.api_client
            .post(format!("http://{}/admin/newsletter", self.address))
            .form(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter(&self) -> Response {
        self.api_client
            .get(format!("http://{}/admin/newsletter", self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    pub async fn create_unconfirmed_subscriber(&self) -> ConfirmationLinks {
        let name: String = Name().fake();
        let email: String = SafeEmail().fake();

        let body = serde_json::json!({
            "name": name,
            "email": email
        });

        let _mock_guard = when_sending_an_email()
            .respond_with(ResponseTemplate::new(200))
            .named("Create unconfirmed subscriber")
            .expect(1)
            .mount_as_scoped(&self.email_server)
            .await;

        let response = self.post_subscriptions(&body).await;
        assert_eq!(response.status(), StatusCode::OK);
        let email_request = self
            .email_server
            .received_requests()
            .await
            .unwrap()
            .pop()
            .unwrap();
        self.get_confirmation_links(&email_request)
    }

    pub async fn create_confirmed_subscriber(&self) {
        let confirmation_links = self.create_unconfirmed_subscriber().await;
        let response = reqwest::get(confirmation_links.html).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
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
        self.api_server.abort();
    }
}

impl TestUser {
    async fn generate(pool: &PgPool) -> Self {
        let username = Uuid::now_v7().to_string();
        let password = Uuid::now_v7().to_string();
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes())
            .expect("Failed to hash the password.")
            .to_string();
        sqlx::query!(
            r#"
            insert into users (username, password_hash)
            values ($1, $2)
            "#,
            username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to create test users.");
        Self { username, password }
    }
}
