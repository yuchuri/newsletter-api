use axum::{Router, routing};

use health_check::*;
use subscriptions::*;
use subscriptions_confirm::*;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

mod health_check;
mod subscriptions;
mod subscriptions_confirm;

pub fn create_route(state: AppState) -> Router {
    Router::new()
        .route("/health_check", routing::get(health_check))
        .route("/subscriptions", routing::post(subscribe))
        .route("/subscriptions/confirm", routing::get(confirm))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
