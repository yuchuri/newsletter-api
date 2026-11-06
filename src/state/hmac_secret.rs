use std::{ops::Deref, sync::Arc};

use secrecy::SecretString;

#[derive(Clone)]
pub struct HmacSecret(Arc<SecretString>);

impl HmacSecret {
    pub fn new(secret: SecretString) -> Self {
        Self(Arc::new(secret))
    }
}

impl Deref for HmacSecret {
    type Target = SecretString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
