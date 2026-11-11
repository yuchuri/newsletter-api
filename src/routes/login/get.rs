use std::fmt::Write;

use axum::response::Html;
use axum_messages::{Level, Messages};

pub async fn login_form(messages: Messages) -> Html<String> {
    let mut error_html = String::new();
    for message in messages.into_iter().filter(|m| m.level == Level::Error) {
        writeln!(error_html, "<p><i>{}</i></p>", message.message).unwrap();
    }

    Html(
        include_str!("login.html")
            .to_string()
            .replace("{{ERROR}}", &error_html),
    )
}
