use std::{env, io, sync::LazyLock, time::Duration};

use argon2::{Argon2, PasswordHasher};
use axum::{Router, body::Body, http::Request, response::Response};
use base64::{Engine, engine::general_purpose};
use linkify::{LinkFinder, LinkKind};
use newsletter_api::{
    configuration, routes,
    state::{AppState, ApplicationBaseUrl, DummyPasswordHash, EmailClient},
};
use reqwest::Url;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use tower::ServiceExt;
use tower_sessions::{SessionManagerLayer, cookie::Key};
use tower_sessions_redis_store::{
    RedisStore,
    fred::{clients::Pool, interfaces::ClientLike, types::config::Config},
};
use tracing_subscriber::{EnvFilter, fmt::writer::BoxMakeWriter};
use uuid::Uuid;
use wiremock::MockServer;

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

pub struct TestContext {
    pub router: Router,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
}

pub struct TestUser {
    pub username: String,
    pub password: String,
}

pub struct ConfirmationLinks {
    pub html: Url,
    pub plain_text: Url,
}

impl TestContext {
    pub async fn new(pool: PgPool) -> Self {
        LazyLock::force(&TRACING);

        let email_server = MockServer::start().await;

        let configuration = configuration::get().expect("Failed to read configuration.");

        let sender_email: newsletter_api::domain::SubscriberEmail = configuration
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
        let session_key = Key::try_from(
            general_purpose::STANDARD
                .decode(configuration.application.hmac_secret.expose_secret())
                .expect("Invalid application HMAC secret.")
                .as_slice(),
        )
        .expect("Invalid application HMAC key size.");

        let redis_pool = Pool::new(
            Config::from_url(configuration.redis_uri.expose_secret()).expect("Invalid Redis URI."),
            None,
            None,
            None,
            6,
        )
        .expect("Failed to create Redis pool.");
        let _redis_connection = redis_pool.connect();
        redis_pool
            .wait_for_connect()
            .await
            .expect("Failed to connect to Redis.");
        let redis_store = RedisStore::new(redis_pool);

        let state = AppState {
            pool: pool.clone(),
            email_client,
            base_url,
            dummy_password_hash: DummyPasswordHash::generate(),
        };

        let router = routes::create_route(state).layer(
            SessionManagerLayer::new(redis_store)
                .with_secure(false)
                .with_signed(session_key),
        );
        let test_user = TestUser::generate(&pool).await;
        Self {
            router,
            pool,
            email_server,
            test_user,
        }
    }

    pub async fn get(&self, uri: &str) -> Response {
        let request = Request::builder()
            .uri(uri)
            .method("GET")
            .body(axum::body::Body::empty())
            .expect("Failed to build request.");

        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_subscriptions(&self, body: &str) -> Response {
        let request = Request::builder()
            .uri("/subscriptions")
            .method("POST")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))
            .expect("Failed to build request.");

        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletter(&self, body: serde_json::Value) -> Response {
        let request = Request::builder()
            .uri("/newsletter")
            .method("POST")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                &format!(
                    "Basic {}",
                    general_purpose::STANDARD.encode(format!(
                        "{}:{}",
                        self.test_user.username, self.test_user.password
                    ))
                ),
            )
            .body(Body::from(
                serde_json::to_vec(&body).expect("Failed to serialize newsletter"),
            ))
            .expect("Failed to build request.");

        self.router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request.")
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
