use axum::response::Html;

pub async fn home() -> Html<String> {
    Html(include_str!("home/home.html").to_string())
}
