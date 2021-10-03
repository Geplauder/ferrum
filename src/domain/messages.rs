use chrono::{DateTime, Utc};
use ferrum_db::{messages::models::MessageModel, users::models::UserModel};
use uuid::Uuid;

use super::users::UserResponse;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user: UserResponse,
    pub content: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl MessageResponse {
    pub fn new(message: &MessageModel, user: &UserModel) -> Self {
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
