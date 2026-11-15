use askama::Template;
use axum::response::Html;
use axum_messages::Messages;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "publish_newsletter_issue.html")]
struct PublishNewsletterTemplate {
    messages: Vec<String>,
    idempotency_key: Uuid,
}

pub async fn publish_newsletter_form(messages: Messages) -> Html<String> {
    let template = PublishNewsletterTemplate {
        messages: messages
            .into_iter()
            .map(|message| message.message)
            .collect(),
        idempotency_key: Uuid::now_v7(),
    };

    Html(template.render().unwrap())
}
