use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::users::queries::{get_users_on_server, is_user_on_server};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum GetUsersError {
    #[error("Forbidden")]
    ForbiddenError,
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
            GetUsersError::ForbiddenError => StatusCode::FORBIDDEN,
            GetUsersError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get server users", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn get_users(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, GetUsersError> {
    let is_user_on_server = is_user_on_server(&pool, auth.claims.id, *server_id)
        .await
        .context("Failed to check if user is on server")?;

    if is_user_on_server == false {
        return Err(GetUsersError::ForbiddenError);
    }

    let server_users = get_users_on_server(*server_id, &pool)
        .await
        .context("Failed to retrieve server users")?;

    Ok(HttpResponse::Ok().json(server_users))
}
