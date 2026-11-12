use std::fmt::Write;

use anyhow::Context;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_messages::Messages;

use crate::{error::AppResult, session_state::TypedSession};

pub async fn change_password_form(
    session: TypedSession,
    messages: Messages,
) -> AppResult<Response> {
    if session
        .get_user_id()
        .await
        .context("Failed to retrieve user_id from session")?
        .is_none()
    {
        return Ok(Redirect::to("/login").into_response());
    }

    let mut msg_html = String::new();
    for message in messages {
        writeln!(
            msg_html,
            "<p><i>{}</i></p>",
            html_escape::encode_text(&message.message)
        )
        .unwrap();
    }
    Ok(
        Html(include_str!("change_password.html").replace("{{MESSAGE}}", &msg_html))
            .into_response(),
    )
}
