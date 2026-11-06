use std::fmt::Debug;

pub type AuthResult<T> = Result<T, AuthError>;

#[derive(thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentialsError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}
