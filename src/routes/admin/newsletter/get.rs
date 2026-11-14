use std::fmt::Write;

use axum::response::Html;
use axum_messages::Messages;

pub async fn publish_newsletter_form(messages: Messages) -> Html<String> {
    let mut msg_html = String::new();
    for message in messages {
        writeln!(
            msg_html,
            "<p><i>{}</i></p>",
            html_escape::encode_text(&message.message)
        )
        .unwrap()
    }

    Html(include_str!("publish_newsletter_issue.html").replace("{{MESSAGE}}", &msg_html))
}
