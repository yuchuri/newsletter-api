use axum::{Router, http::Request, routing};

use health_check::*;
use newsletter::*;
use subscriptions::*;
use subscriptions_confirm::*;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::state::AppState;

mod health_check;
mod newsletter;
mod subscriptions;
mod subscriptions_confirm;

pub fn create_route(state: AppState) -> Router {
    Router::new()
        .route("/health_check", routing::get(health_check))
        .route("/subscriptions", routing::post(subscribe))
        .route("/subscriptions/confirm", routing::get(confirm))
        .route("/newsletter", routing::post(publish_newsletter))
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
        .with_state(state)
}
