use validator::ValidateEmail;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl TryFrom<String> for SubscriberEmail {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.validate_email() {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid subscriber email.", value))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;
    use fake::{Fake, faker::internet::en::SafeEmail};
    use proptest::prelude::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::try_from(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::try_from(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::try_from(email));
    }

    fn fake_email_strategy() -> impl Strategy<Value = String> {
        any::<u64>().prop_map(|seed| {
            let mut rng = StdRng::seed_from_u64(seed);
            SafeEmail().fake_with_rng(&mut rng)
        })
    }

    proptest! {
        #[test]
        fn valid_emails_are_parsed_successfully(valid_email in fake_email_strategy()) {
            prop_assert!(SubscriberEmail::try_from(valid_email).is_ok());
        }
    }
}
