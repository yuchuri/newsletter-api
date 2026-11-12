use anyhow::Context;
use axum::response::Redirect;
use axum_messages::Messages;

use crate::{error::AppResult, session_state::TypedSession};

pub async fn logout(session: TypedSession, messages: Messages) -> AppResult<Redirect> {
    if session
        .get_user_id()
        .await
        .context("Failed to retrieve user_id from session")?
        .is_none()
    {
        Ok(Redirect::to("/login"))
    } else {
        session.logout().await.context("Failed to logout user")?;
        messages.info("You have successfully logged out.");
        Ok(Redirect::to("/login"))
    }
}
