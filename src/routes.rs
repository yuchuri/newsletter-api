use axum::{Router, routing};

use health_check::*;
use subscriptions::*;

mod health_check;
mod subscriptions;

pub fn create_route() -> Router {
    Router::new()
        .route("/health_check", routing::get(health_check))
        .route("/subscriptions", routing::post(subscribe))
}
