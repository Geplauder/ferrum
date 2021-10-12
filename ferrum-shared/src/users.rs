use chrono::{DateTime, Utc};
use uuid::Uuid;

///
/// Model for users that does not contain sensitive information and can be used for responses.
///
/// In this case, this omits the users' email and password.
///
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
