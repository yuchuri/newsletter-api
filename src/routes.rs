use axum::{Router, http::Request, middleware, routing};
use axum_messages::MessagesManagerLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{authentication::reject_anonymous_users, state::AppState};

mod admin;
mod health_check;
mod home;
mod login;
mod subscriptions;
mod subscriptions_confirm;

use admin::*;
use health_check::*;
use home::*;
use login::*;
use subscriptions::*;
use subscriptions_confirm::*;

pub fn create_router(state: AppState) -> Router {
    let admin_router = Router::new()
        .route("/dashboard", routing::get(admin_dashboard))
        .route("/logout", routing::post(logout))
        .route(
            "/newsletter",
            routing::get(publish_newsletter_form).post(publish_newsletter),
        )
        .route(
            "/password",
            routing::get(change_password_form).post(change_password),
        )
        .route_layer(middleware::from_fn(reject_anonymous_users));

    Router::new()
        .route("/", routing::get(home))
        .route("/health_check", routing::get(health_check))
        .route("/login", routing::get(login_form).post(login))
        .route("/subscriptions", routing::post(subscribe))
        .route("/subscriptions/confirm", routing::get(confirm))
        .nest("/admin", admin_router)
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
        .with_state(state)
}
