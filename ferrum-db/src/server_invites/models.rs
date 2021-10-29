use chrono::{DateTime, Utc};
use ferrum_shared::server_invites::ServerInviteResponse;
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

impl From<ServerInviteModel> for ServerInviteResponse {
    fn from(val: ServerInviteModel) -> Self {
        ServerInviteResponse {
            id: val.id,
            server_id: val.server_id,
            code: val.code,
            updated_at: val.updated_at,
            created_at: val.created_at,
        }
    }
}
