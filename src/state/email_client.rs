use std::{sync::Arc, time::Duration};

use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use crate::domain::SubscriberEmail;

#[derive(Clone)]
pub struct EmailClient {
    inner: Arc<EmailClientInner>,
}

struct EmailClientInner {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: SecretString,
        timeout: Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            inner: Arc::new(EmailClientInner {
                http_client,
                base_url,
                sender,
                authorization_token,
            }),
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.inner.base_url);
        let request_body = SendEmailRequest {
            from: self.inner.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        let _builder = self
            .inner
            .http_client
            .post(&url)
            .header(
                "X-Postmark-Server-Token",
                self.inner.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claim::{assert_err, assert_ok};
    use fake::{
        Fake, Faker,
        faker::{internet::en::SafeEmail, lorem::en::Sentence},
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    use super::*;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            if let Ok(body) = serde_json::from_slice::<serde_json::Value>(&request.body) {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    // Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    // Generatea a random email content
    fn content() -> String {
        Sentence(1..10).fake()
    }

    // Generate a random subscriber email
    fn email() -> SubscriberEmail {
        SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap()
    }

    // Get a test instance of the `EmailClient`.
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            SecretString::new(Faker.fake::<String>().into()),
            Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let subscriber_email = SubscriberEmail::try_from(SafeEmail().fake::<String>()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Sentence(1..10).fake();

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(180)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(outcome);
    }
}
