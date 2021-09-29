use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct NewChannel {
    pub name: ChannelName,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub struct ChannelName(String);

impl ChannelName {
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
    use fake::Fake;

    #[test]
    fn name_too_short_is_rejected() {
        for x in ["", "abc"] {
            let name = x.to_string();

            assert!(ChannelName::parse(name).is_err());
        }
    }

    #[test]
    fn name_too_long_is_rejected() {
        let name = (0..=33).map(|_| "x").collect::<String>();

        assert!(ChannelName::parse(name).is_err());
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
