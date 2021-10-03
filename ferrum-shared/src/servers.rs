use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
