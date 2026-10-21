use axum::{Router, routing};

use health_check::*;

mod health_check;

pub fn create_route() -> Router {
    Router::new().route("/health_check", routing::get(health_check))
}
