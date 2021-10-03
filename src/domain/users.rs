use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
