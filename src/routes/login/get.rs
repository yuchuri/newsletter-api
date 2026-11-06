use axum::{
    extract::{Query, State, rejection::QueryRejection},
    response::Html,
};
use hmac::{KeyInit, Mac};
use percent_encoding::NON_ALPHANUMERIC;
use secrecy::ExposeSecret;
use serde::Deserialize;

use crate::state::HmacSecret;

type HmacSha256 = hmac::Hmac<sha2::Sha256>;

#[derive(Deserialize)]
pub struct LoginQuery {
    pub error: String,
    pub tag: String,
}

impl LoginQuery {
    fn verify(self, secret: &HmacSecret) -> anyhow::Result<String> {
        let tag = hex::decode(self.tag)?;
        let encoded_error =
            percent_encoding::utf8_percent_encode(&self.error, NON_ALPHANUMERIC).to_string();
        let query = format!("error={encoded_error}");

        let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes()).unwrap();
        mac.update(query.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

pub async fn login_form(
    request: Result<Query<LoginQuery>, QueryRejection>,
    State(secret): State<HmacSecret>,
) -> Html<String> {
    let error_html = match request {
        Ok(Query(request)) => match request.verify(&secret) {
            Ok(error) => format!(r#"<p><i>{}</i></p>"#, html_escape::encode_text(&error)),
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.chain = ?e,
                    "Failed to verify query parameters using the HMAC tag"
                );
                String::new()
            }
        },
        Err(_) => String::new(),
    };
    Html(
        include_str!("login.html")
            .to_string()
            .replace("{{ERROR}}", &error_html),
    )
}
