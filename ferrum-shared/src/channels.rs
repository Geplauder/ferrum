use chrono::{DateTime, Utc};
use uuid::Uuid;

///
/// Model for channels that does not contain sensitive information and can be used for responses.
///
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelResponse {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
