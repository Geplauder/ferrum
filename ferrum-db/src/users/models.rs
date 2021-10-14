use anyhow::Context;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Utc};
use ferrum_shared::users::UserResponse;
use uuid::Uuid;

///
/// Check if the given password matches with the expected password hash.
///
/// Note: This has to be executed in a new thread, to prevent the blocking.
///
#[tracing::instrument(
    name = "Verify credentials",
    skip(expected_password_hash, given_password)
)]
pub fn verify_password_hash(
    expected_password_hash: String,
    given_password: String,
) -> Result<bool, anyhow::Error> {
    let expected_password_hash =
        PasswordHash::new(&expected_password_hash).context("Failed to parse password hash.")?;

    // Check the given password against the stored one
    if Argon2::default()
        .verify_password(given_password.as_bytes(), &expected_password_hash)
        .is_err()
    {
        return Ok(false);
    }

    Ok(true)
}

///
/// Contains validated data to create a new user.
///
pub struct NewUser {
    pub name: UserName,
    pub email: UserEmail,
    pub password: UserPassword,
}

///
/// Contains validated data to update an existing user.
///
pub struct UpdateUser {
    pub name: Option<UserName>,
    pub email: Option<UserEmail>,
    pub password: Option<UserPassword>,
    pub current_password: String,
}

///
/// Model to fetch a user from the database with.
///
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct UserModel {
    pub id: Uuid,
    pub username: String,
    pub password: String,
    pub email: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<UserModel> for UserResponse {
    fn from(val: UserModel) -> Self {
        Self {
            id: val.id,
            username: val.username,
            updated_at: val.updated_at,
            created_at: val.created_at,
        }
    }
}

///
/// Provides a validated user email.
///
#[derive(Debug)]
pub struct UserEmail(String);

impl UserEmail {
    ///
    /// Parse a [`UserEmail`] from a [`String`].
    ///
    /// This ensures that it is fully validated.
    ///
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

///
/// Provides a validated user name.
///
#[derive(Debug)]
pub struct UserName(String);

impl UserName {
    ///
    /// Parse a [UserName] from a [String].
    ///
    /// This ensures that it is fully validated.
    ///
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

///
/// Provides a validated user password.
///
#[derive(Debug)]
pub struct UserPassword(String);

impl UserPassword {
    ///
    /// Parse a [UserPassword] from a [String].
    ///
    /// This ensures that it is fully validated.
    ///
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
    use claim::assert_err;
    use fake::faker::internet::en::{Password, SafeEmail, Username};
    use fake::Fake;

    #[test]
    fn email_empty_string_is_rejected() {
        let email = "".to_string();

        assert_err!(UserEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "foobar.com".to_string();

        assert_err!(UserEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@bar.com".to_string();

        assert_err!(UserEmail::parse(email));
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

            assert_err!(UserName::parse(name));
        }
    }

    #[test]
    fn name_too_long_is_rejected() {
        let name = (0..=33).map(|_| "x").collect::<String>();

        assert_err!(UserName::parse(name));
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

            assert_err!(UserPassword::parse(pasword));
        }
    }

    #[test]
    fn passsword_too_long_is_rejected() {
        let password = (0..=65).map(|_| "x").collect::<String>();

        assert_err!(UserPassword::parse(password));
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
