use chrono::{DateTime, Utc};
use uuid::Uuid;

///
/// Model to fetch a server invite from the database with.
///
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ServerInviteModel {
    pub id: Uuid,
    pub server_id: Uuid,
    pub code: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
