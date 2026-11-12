use std::fmt::Write;

use axum::response::Html;
use axum_messages::Messages;

pub async fn login_form(messages: Messages) -> Html<String> {
    let mut error_html = String::new();
    for message in messages {
        writeln!(
            error_html,
            "<p><i>{}</i></p>",
            html_escape::encode_text(&message.message)
        )
        .unwrap();
    }

    Html(
        include_str!("login.html")
            .to_string()
            .replace("{{ERROR}}", &error_html),
    )
}
