use axum::{Router, http::Request, routing};
use axum_messages::MessagesManagerLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::state::AppState;

mod admin;
mod health_check;
mod home;
mod login;
mod newsletter;
mod subscriptions;
mod subscriptions_confirm;

use admin::*;
use health_check::*;
use home::*;
use login::*;
use newsletter::*;
use subscriptions::*;
use subscriptions_confirm::*;

pub fn create_route(state: AppState) -> Router {
    Router::new()
        .route("/", routing::get(home))
        .route("/health_check", routing::get(health_check))
        .route("/login", routing::get(login_form).post(login))
        .route("/subscriptions", routing::post(subscribe))
        .route("/subscriptions/confirm", routing::get(confirm))
        .route("/newsletter", routing::post(publish_newsletter))
        .route("/admin/dashboard", routing::get(admin_dashboard))
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
