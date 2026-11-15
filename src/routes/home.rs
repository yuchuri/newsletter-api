use axum::response::Html;

pub async fn home() -> Html<String> {
    Html(include_str!("../../templates/home.html").to_string())
}
