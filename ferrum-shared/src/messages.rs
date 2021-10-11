use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::users::UserResponse;

///
/// Model for messages that does not contain sensitive information and can be used for responses.
///
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user: UserResponse,
    pub content: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
