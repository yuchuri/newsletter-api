use std::{fmt::Display, ops::Deref};

use axum::{extract::Request, middleware::Next, response::Response};
use axum_messages::Messages;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    session_state::TypedSession,
};

#[derive(Clone, Copy, Debug)]
pub struct UserId(Uuid);

impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserId {
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

#[tracing::instrument(
    skip_all,
    fields(user_id = tracing::field::Empty),
)]
pub async fn reject_anonymous_users(
    session: TypedSession,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    match session.get_user_id().await? {
        Some(user_id) => {
            tracing::Span::current().record("user_id", tracing::field::display(&user_id));
            request.extensions_mut().insert(UserId(user_id));
            Ok(next.run(request).await)
        }
        None => {
            let err = AppError::BasicAuth(anyhow::anyhow!("The user is not logged in."));
            if let Some(messages) = request.extensions().get::<Messages>().cloned() {
                messages.error(err.to_string());
            }

            Err(err.redirect("/login"))
        }
    }
}
