use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::users::{User, UserResponse};

pub struct NewMessage {
    pub content: MessageContent,
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user: UserResponse,
    pub content: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl MessageResponse {
    pub fn new(message: &Message, user: &User) -> Self {
        Self {
            id: message.id,
            channel_id: message.channel_id,
            user: UserResponse {
                id: user.id,
                username: user.username.to_owned(),
                updated_at: user.updated_at,
                created_at: user.created_at,
            },
            content: message.content.to_owned(),
            updated_at: message.updated_at,
            created_at: message.created_at,
        }
    }
}

pub struct MessageContent(String);

impl MessageContent {
    pub fn parse(value: String) -> Result<MessageContent, String> {
        if validator::validate_length(&value, Some(1), Some(1000), None) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not a valid message content!", value))
        }
    }
}

impl AsRef<str> for MessageContent {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::MessageContent;
    use fake::Fake;

    #[test]
    fn content_to_short_is_rejected() {
        assert!(MessageContent::parse("".to_string()).is_err());
    }

    #[test]
    fn content_too_long_is_rejected() {
        let content = (0..=1001).map(|_| "x").collect::<String>();

        assert!(MessageContent::parse(content).is_err());
    }

    #[test]
    fn content_display_trait_implementation_is_valid() {
        let content = "foobar".to_string();

        let message_content = MessageContent::parse(content).unwrap();

        assert_eq!("foobar", message_content.to_string());
    }

    #[derive(Debug, Clone)]
    struct ValidContentFixture(pub String);

    impl quickcheck::Arbitrary for ValidContentFixture {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let content = (1..1001).fake_with_rng::<String, G>(g);

            Self(content)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn valid_content_is_parsed_successfully(valid_content: ValidContentFixture) -> bool {
        MessageContent::parse(valid_content.0).is_ok()
    }
}
