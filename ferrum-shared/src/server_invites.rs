use chrono::{DateTime, Utc};
use uuid::Uuid;

///
/// Model for server invites that does not contain sensitive information and can be used for responses.
///
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerInviteResponse {
    pub id: Uuid,
    pub server_id: Uuid,
    pub code: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
