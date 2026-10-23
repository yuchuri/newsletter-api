use axum::{Router, routing};

use health_check::*;
use subscriptions::*;

use crate::state::AppState;

mod health_check;
mod subscriptions;

pub fn create_route(state: AppState) -> Router {
    Router::new()
        .route("/health_check", routing::get(health_check))
        .route("/subscriptions", routing::post(subscribe))
        .with_state(state)
}
