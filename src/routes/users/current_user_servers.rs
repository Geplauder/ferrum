use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::servers::Server, error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum CurrentUserServersError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CurrentUserServersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for CurrentUserServersError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            CurrentUserServersError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get current user servers", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn current_user_servers(
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CurrentUserServersError> {
    let user_servers = get_user_servers(auth.claims.id, &pool).await?;

    Ok(HttpResponse::Ok().json(user_servers))
}

#[tracing::instrument(name = "Get store user servers", skip(user_id, pool))]
async fn get_user_servers(
    user_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<Server>, CurrentUserServersError> {
    let servers = sqlx::query_as!(
        Server,
        r#"
        SELECT servers.*
        FROM users_servers
        INNER JOIN servers ON users_servers.server_id = servers.id
        WHERE users_servers.user_id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to retrieve user servers")?;

    Ok(servers)
}
