use askama::Template;
use axum::response::Html;
use axum_messages::Messages;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    messages: Vec<String>,
}

pub async fn login_form(messages: Messages) -> Html<String> {
    let template = LoginTemplate {
        messages: messages
            .into_iter()
            .map(|message| message.message)
            .collect(),
    };

    Html(template.render().unwrap())
}
