use axum::{Router, extract::Request};
use axum_messages::MessagesManagerLayer;
use base64::{Engine, engine::general_purpose};
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_sessions::{SessionManagerLayer, cookie::Key};
use tower_sessions_redis_store::{
    RedisStore,
    fred::{clients::Pool, interfaces::ClientLike, types::config::Config},
};
use uuid::Uuid;

use crate::state::{ApplicationBaseUrl, DummyPasswordHash};

use super::{configuration::Settings, routes, state::AppState};

pub struct Application {
    port: u16,
    listener: TcpListener,
    router: Router,
}

impl Application {
    pub async fn run(self) -> anyhow::Result<()> {
        axum::serve(self.listener, self.router).await?;
        Ok(())
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn build(configuration: Settings) -> anyhow::Result<Self> {
        let configuration_pool = PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(configuration.database.connection_options());

        let email_client = configuration.email_client.client();

        let base_url = ApplicationBaseUrl::new(configuration.application.base_url);
        let dummy_password_hash = DummyPasswordHash::generate();
        let session_key = Key::try_from(
            general_purpose::STANDARD
                .decode(configuration.application.hmac_secret.expose_secret())?
                .as_slice(),
        )?;

        let redis_pool = Pool::new(
            Config::from_url(configuration.redis_uri.expose_secret())?,
            None,
            None,
            None,
            6,
        )?;
        let _redis_connection = redis_pool.connect();
        redis_pool.wait_for_connect().await?;
        let redis_store = RedisStore::new(redis_pool);

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address).await?;
        let port = listener.local_addr()?.port();

        let state = AppState {
            pool: configuration_pool,
            email_client,
            base_url,
            dummy_password_hash,
        };

        let router = routes::create_router()
            .layer(
                TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = %Uuid::now_v7(),
                    )
                }),
            )
            .layer(MessagesManagerLayer)
            .layer(
                SessionManagerLayer::new(redis_store)
                    .with_secure(false)
                    .with_signed(session_key),
            )
            .with_state(state);

        Ok(Self {
            port,
            listener,
            router,
        })
    }
}
