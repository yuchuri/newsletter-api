use axum::http::StatusCode;

pub(super) async fn health_check() -> StatusCode {
    StatusCode::OK
}
