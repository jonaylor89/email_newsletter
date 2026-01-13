#[derive(Debug, Clone)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    /// Parses a string into a valid SubscriptionToken.
    ///
    /// Validation rules:
    /// - Must be exactly 25 characters long
    /// - Must contain only alphanumeric characters (a-z, A-Z, 0-9)
    pub fn parse(s: String) -> Result<Self, String> {
        let is_valid_length = s.len() == 25;
        let is_alphanumeric = s.chars().all(|c| c.is_ascii_alphanumeric());

        if !is_valid_length {
            Err(format!(
                "Subscription token must be exactly 25 characters long, got {}",
                s.len()
            ))
        } else if !is_alphanumeric {
            Err("Subscription token must contain only alphanumeric characters".to_string())
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriptionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriptionToken;
    use claim::{assert_err, assert_ok};

    #[test]
    fn token_with_correct_length_and_alphanumeric_is_valid() {
        let token = "a".repeat(25);
        assert_ok!(SubscriptionToken::parse(token));
    }

    #[test]
    fn token_shorter_than_25_chars_is_rejected() {
        let token = "tooshort".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn token_longer_than_25_chars_is_rejected() {
        let token = "a".repeat(26);
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn token_with_special_characters_is_rejected() {
        let mut token = "a".repeat(24);
        token.push('!');
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn token_with_spaces_is_rejected() {
        let mut token = "a".repeat(24);
        token.push(' ');
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn empty_string_is_rejected() {
        let token = "".to_string();
        assert_err!(SubscriptionToken::parse(token));
    }

    #[test]
    fn valid_alphanumeric_token_is_accepted() {
        let token = "aBc123XyZ456mNoPqR789stUV".to_string();
        assert_ok!(SubscriptionToken::parse(token));
    }
}
