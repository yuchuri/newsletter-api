use axum::response::Html;

pub async fn login_form() -> Html<String> {
    Html(include_str!("login.html").to_string())
}
