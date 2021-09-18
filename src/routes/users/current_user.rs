use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::users::User, error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum UsersError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for UsersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UsersError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            UsersError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get current user information", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn current_user(
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, UsersError> {
    let user_data = get_user_data(auth.claims.id, &pool).await?;

    Ok(HttpResponse::Ok().json(user_data))
}

#[tracing::instrument(name = "Get stored user", skip(user_id, pool))]
async fn get_user_data(user_id: Uuid, pool: &PgPool) -> Result<User, UsersError> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, updated_at, created_at
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to retrieve stored user.")?;

    Ok(user)
}
