use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::models::ServerModel;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error_chain_fmt, jwt::AuthorizationService, utilities::is_user_on_server};

#[derive(thiserror::Error)]
pub enum GetServerError {
    #[error("Forbidden")]
    ForbiddenError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetServerError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            GetServerError::ForbiddenError => StatusCode::FORBIDDEN,
            GetServerError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument("Get server", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn get(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, GetServerError> {
    let is_user_on_server = is_user_on_server(&pool, auth.claims.id, *server_id)
        .await
        .context("Failed to check if user is on server")?;

    if is_user_on_server == false {
        return Err(GetServerError::ForbiddenError);
    }

    let server = get_server(*server_id, &pool)
        .await
        .context("Error while fetching server")?;

    Ok(HttpResponse::Ok().json(server))
}

#[tracing::instrument(name = "Get server", skip(server_id, pool))]
async fn get_server(server_id: Uuid, pool: &PgPool) -> Result<ServerModel, sqlx::Error> {
    let server = sqlx::query_as!(
        ServerModel,
        r#"
        SELECT *
        FROM servers
        WHERE servers.id = $1
        "#,
        server_id
    )
    .fetch_one(pool)
    .await?;

    Ok(server)
}
