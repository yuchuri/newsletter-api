use axum::response::Redirect;
use axum_messages::Messages;

use crate::{error::AppResult, session_state::TypedSession};

#[tracing::instrument(name = "Log out user", skip_all)]
pub async fn logout(session: TypedSession, messages: Messages) -> AppResult<Redirect> {
    session.logout().await?;
    messages.info("You have successfully logged out.");
    Ok(Redirect::to("/login"))
}
