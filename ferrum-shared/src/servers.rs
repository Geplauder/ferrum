use chrono::{DateTime, Utc};
use uuid::Uuid;

///
/// Model for servers that does not contain sensitive information and can be used for responses.
///
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub flags: u32,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
