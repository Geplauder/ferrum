use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelResponse {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
