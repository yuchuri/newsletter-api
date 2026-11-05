use std::{ops::Deref, sync::Arc};

use argon2::{Argon2, PasswordHasher};
use secrecy::SecretString;
use uuid::Uuid;

#[derive(Clone)]
pub struct DummyPasswordHash(Arc<SecretString>);

impl DummyPasswordHash {
    pub fn generate() -> Self {
        let inner = Argon2::default()
            .hash_password(Uuid::now_v7().as_bytes())
            .expect("Failed to generate a dummy password hash.")
            .to_string();
        Self(Arc::new(SecretString::from(inner)))
    }
}

impl Deref for DummyPasswordHash {
    type Target = SecretString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
