use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::users::User, error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum GetUsersError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetUsersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetUsersError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            GetUsersError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn get_users(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    _auth: AuthorizationService,
) -> Result<HttpResponse, GetUsersError> {
    // TODO: Check if authenticated user is on server
    let server_users = get_server_users(*server_id, &pool).await?;

    Ok(HttpResponse::Ok().json(server_users))
}

async fn get_server_users(server_id: Uuid, pool: &PgPool) -> Result<Vec<User>, GetUsersError> {
    let users = sqlx::query_as!(
        User,
        r#"
        SELECT users.id, users.username, users.email, users.created_at, users.updated_at
        FROM users_servers
        INNER JOIN users ON users_servers.user_id = users.id
        WHERE users_servers.server_id = $1
        "#,
        server_id,
    )
    .fetch_all(pool)
    .await
    .context("Failed to retrieve server users")?;

    Ok(users)
}
