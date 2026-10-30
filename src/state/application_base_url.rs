use std::{fmt::Display, ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct ApplicationBaseUrl(Arc<String>);

impl ApplicationBaseUrl {
    pub fn new(base_url: String) -> Self {
        Self(Arc::new(base_url))
    }
}

impl Deref for ApplicationBaseUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ApplicationBaseUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
