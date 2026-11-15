use askama::Template;
use axum::response::Html;
use axum_messages::Messages;

#[derive(Template)]
#[template(path = "change_password.html")]
struct ChangePasswordTemplate {
    messages: Vec<String>,
}

pub async fn change_password_form(messages: Messages) -> Html<String> {
    let template = ChangePasswordTemplate {
        messages: messages
            .into_iter()
            .map(|message| message.message)
            .collect(),
    };

    Html(template.render().unwrap())
}
