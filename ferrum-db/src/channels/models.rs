use chrono::{DateTime, Utc};
use ferrum_shared::channels::ChannelResponse;
use uuid::Uuid;

///
/// Contains validated data to create a new channel.
///
pub struct NewChannel {
    pub name: ChannelName,
}

///
/// Model to fetch a channel from the database with.
///
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ChannelModel {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<ChannelModel> for ChannelResponse {
    fn from(val: ChannelModel) -> Self {
        ChannelResponse {
            id: val.id,
            server_id: val.server_id,
            name: val.name,
            updated_at: val.updated_at,
            created_at: val.created_at,
        }
    }
}

///
/// Provides a validated channel name.
///
#[derive(Debug)]
pub struct ChannelName(String);

impl ChannelName {
    ///
    /// Parse a [`ChannelName`] from a [`String`].
    ///
    /// This ensures that it is fully validated.
    ///
    pub fn parse(value: String) -> Result<ChannelName, String> {
        if validator::validate_length(&value, Some(4), Some(32), None) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid channel name!", value))
        }
    }
}

impl AsRef<str> for ChannelName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChannelName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::ChannelName;
    use claim::assert_err;
    use fake::Fake;

    #[test]
    fn name_too_short_is_rejected() {
        for x in ["", "abc"] {
            let name = x.to_string();

            assert_err!(ChannelName::parse(name));
        }
    }

    #[test]
    fn name_too_long_is_rejected() {
        let name = (0..=33).map(|_| "x").collect::<String>();

        assert_err!(ChannelName::parse(name));
    }

    #[test]
    fn name_display_trait_implementation_is_valid() {
        let name = "foobar".to_string();

        let channel_name = ChannelName::parse(name).unwrap();

        assert_eq!("foobar", channel_name.to_string());
    }

    #[derive(Debug, Clone)]
    struct ValidNameFixture(pub String);

    impl quickcheck::Arbitrary for ValidNameFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let name = (4..33).fake_with_rng::<String, G>(g);

            Self(name)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_names_are_parsed_successfully(valid_name: ValidNameFixture) -> bool {
        ChannelName::parse(valid_name.0).is_ok()
    }
}
