use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::rngs::OsRng;

pub struct NewUser {
    pub name: UserName,
    pub email: UserEmail,
    pub password: UserPassword,
}

pub struct UserEmail(String);

impl UserEmail {
    pub fn parse(value: String) -> Result<UserEmail, String> {
        if validator::validate_email(&value) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid user email!", value))
        }
    }
}

impl AsRef<str> for UserEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UserEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct UserName(String);

impl UserName {
    pub fn parse(value: String) -> Result<UserName, String> {
        if validator::validate_length(&value, Some(3), Some(32), None) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid user name!", value))
        }
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct UserPassword(String);

impl UserPassword {
    pub fn parse(value: String) -> Result<UserPassword, String> {
        if validator::validate_length(&value, Some(8), Some(64), None) {
            let argon = Argon2::default();
            let salt = SaltString::generate(&mut OsRng);

            match argon.hash_password(value.as_bytes(), &salt) {
                Ok(value) => Ok(Self(value.to_string())),
                Err(_) => Err("Hash could not be calculated!".to_string()),
            }
        } else {
            Err("Invalid user password!".to_string())
        }
    }
}

impl AsRef<str> for UserPassword {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{UserEmail, UserName, UserPassword};
    use fake::faker::internet::en::{Password, SafeEmail, Username};
    use fake::Fake;

    #[test]
    fn email_empty_string_is_rejected() {
        let email = "".to_string();

        assert!(UserEmail::parse(email).is_err());
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "foobar.com".to_string();

        assert!(UserEmail::parse(email).is_err());
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@bar.com".to_string();

        assert!(UserEmail::parse(email).is_err());
    }

    #[test]
    fn email_display_trait_implementation_is_valid() {
        let email = "foo@bar.com".to_string();

        let user_email = UserEmail::parse(email).unwrap();

        assert_eq!("foo@bar.com", user_email.to_string());
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);

            Self(email)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        UserEmail::parse(valid_email.0).is_ok()
    }

    #[test]
    fn name_too_short_is_rejected() {
        for x in ["", "ab"] {
            let name = x.to_string();

            assert!(UserName::parse(name).is_err());
        }
    }

    #[test]
    fn name_too_long_is_rejected() {
        let name = (0..=33).map(|_| "x").collect::<String>();

        assert!(UserName::parse(name).is_err());
    }

    #[test]
    fn name_display_trait_implementation_is_valid() {
        let name = "foobar".to_string();

        let user_name = UserName::parse(name).unwrap();

        assert_eq!("foobar", user_name.to_string());
    }

    #[derive(Debug, Clone)]
    struct ValidNameFixture(pub String);

    impl quickcheck::Arbitrary for ValidNameFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let name = Username().fake_with_rng(g);

            Self(name)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_names_are_parsed_successfully(valid_name: ValidNameFixture) -> bool {
        UserName::parse(valid_name.0).is_ok()
    }

    #[test]
    fn password_too_short_is_rejected() {
        for x in ["", "abc", "foobar"] {
            let pasword = x.to_string();

            assert!(UserPassword::parse(pasword).is_err());
        }
    }

    #[test]
    fn passsword_too_long_is_rejected() {
        let password = (0..=65).map(|_| "x").collect::<String>();

        assert!(UserPassword::parse(password).is_err());
    }

    #[derive(Debug, Clone)]
    struct ValidPasswordFixture(pub String);

    impl quickcheck::Arbitrary for ValidPasswordFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let password = Password(8..64 + 1).fake_with_rng(g);

            Self(password)
        }
    }
}
