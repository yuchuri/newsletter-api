use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AuthError, AuthResult},
    state::DummyPasswordHash,
    telemetry,
};

pub struct Credentials {
    username: String,
    password: SecretString,
}

impl Credentials {
    pub fn new(username: String, password: SecretString) -> Self {
        Self { username, password }
    }
}

#[tracing::instrument(name = "Validate credentials", skip_all)]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
    dummy_password_hash: &DummyPasswordHash,
) -> AuthResult<Uuid> {
    let (user_id, expected_password_hash) =
        match get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(AuthError::UnexpectedError)?
        {
            Some((user_id, password_hash)) => (Some(user_id), password_hash),
            None => (
                None,
                SecretString::from(dummy_password_hash.expose_secret()),
            ),
        };

    telemetry::spawn_tracing_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Invalid credentials")
    .map_err(AuthError::InvalidCredentialsError)??;

    user_id
        .ok_or_else(|| AuthError::InvalidCredentialsError(anyhow::anyhow!("Invalid credentials")))
}

#[tracing::instrument(name = "Get stored credentials", skip_all)]
pub async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> anyhow::Result<Option<(Uuid, SecretString)>> {
    let row = sqlx::query!(
        r#"
        select user_id, password_hash
        from users
        where username = $1
        "#,
        username,
    )
    .fetch_optional(pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.user_id, SecretString::from(row.password_hash)));

    Ok(row)
}

#[tracing::instrument(name = "Verify password hash", skip_all)]
pub fn verify_password_hash(
    expected_password_hash: SecretString,
    password_candidate: SecretString,
) -> AuthResult<()> {
    let exprected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &exprected_password_hash,
        )
        .context("Invalid password")
        .map_err(AuthError::InvalidCredentialsError)
}

#[tracing::instrument(name = "Change password", skip_all)]
pub async fn change_password(
    user_id: Uuid,
    password: SecretString,
    pool: &PgPool,
) -> anyhow::Result<()> {
    let password_hash = tokio::task::spawn_blocking(move || compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;
    sqlx::query!(
        r#"
        update users
        set password_hash = $1
        where user_id = $2
        "#,
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;
    Ok(())
}

fn compute_password_hash(password: SecretString) -> anyhow::Result<SecretString> {
    Ok(SecretString::from(
        Argon2::default()
            .hash_password(password.expose_secret().as_bytes())
            .context("Failed to hash password")?
            .to_string(),
    ))
}
