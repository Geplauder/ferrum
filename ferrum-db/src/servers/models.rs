use chrono::{DateTime, Utc};
use ferrum_shared::servers::ServerResponse;
use uuid::Uuid;

///
/// Contains validated data to create a new server.
pub struct NewServer {
    pub name: ServerName,
}

///
/// Contains validated data to update an existing server.
///
pub struct UpdateServer {
    pub name: Option<ServerName>,
}

///
/// Model to fetch a server from the database with.
///
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ServerModel {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<ServerModel> for ServerResponse {
    fn from(val: ServerModel) -> Self {
        Self {
            id: val.id,
            name: val.name,
            owner_id: val.owner_id,
            updated_at: val.updated_at,
            created_at: val.created_at,
        }
    }
}

///
/// Provides a validated server name.
///
#[derive(Debug)]
pub struct ServerName(String);

impl ServerName {
    ///
    /// Parse a [`ServerName`] from a [`String`].
    ///
    /// This ensures that it is fully validated and trimmed.
    ///
    pub fn parse(value: String) -> Result<ServerName, String> {
        let value = value.trim();

        if validator::validate_length(value, Some(4), Some(64), None) {
            Ok(Self(value.to_string()))
        } else {
            Err(format!("{} is not a valid server name!", value.to_string()))
        }
    }
}

impl AsRef<str> for ServerName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::ServerName;
    use claim::assert_err;
    use fake::Fake;

    #[test]
    fn name_too_short_is_rejected() {
        for x in ["", "abc"] {
            let name = x.to_string();

            assert_err!(ServerName::parse(name));
        }
    }

    #[test]
    fn name_too_long_is_rejected() {
        let name = (0..=65).map(|_| "x").collect::<String>();

        assert_err!(ServerName::parse(name));
    }

    #[test]
    fn name_display_trait_implementation_is_valid() {
        let name = "foobar".to_string();

        let server_name = ServerName::parse(name).unwrap();

        assert_eq!("foobar", server_name.to_string());
    }

    #[derive(Debug, Clone)]
    struct ValidNameFixture(pub String);

    impl quickcheck::Arbitrary for ValidNameFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let name = (4..65).fake_with_rng::<String, G>(g);

            Self(name)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_names_are_parsed_successfully(valid_name: ValidNameFixture) -> bool {
        ServerName::parse(valid_name.0).is_ok()
    }
}
