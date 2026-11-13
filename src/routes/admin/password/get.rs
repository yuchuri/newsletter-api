use std::fmt::Write;

use axum::response::Html;
use axum_messages::Messages;

pub async fn change_password_form(messages: Messages) -> Html<String> {
    let mut msg_html = String::new();
    for message in messages {
        writeln!(
            msg_html,
            "<p><i>{}</i></p>",
            html_escape::encode_text(&message.message)
        )
        .unwrap();
    }
    Html(include_str!("change_password.html").replace("{{MESSAGE}}", &msg_html))
}
