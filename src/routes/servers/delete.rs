use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::queries::{delete_server, is_user_owner_of_server};
use ferrum_shared::{error_chain_fmt, jwt::AuthorizationService};
use ferrum_websocket::{messages, WebSocketServer};
use sqlx::PgPool;
use uuid::Uuid;

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum DeleteServerError {
    /// User has no permissions to delete a server.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for DeleteServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for DeleteServerError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            DeleteServerError::ForbiddenError => StatusCode::FORBIDDEN,
            DeleteServerError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument("Delete server", skip(pool, /*websocket_server,*/ auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn delete(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    // websocket_server: web::Data<Addr<WebSocketServer>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, DeleteServerError> {
    // Check if the user is the owner of this server, return forbidden error if not
    let is_user_owner = is_user_owner_of_server(&pool, *server_id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(DeleteServerError::ForbiddenError);
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool.")?;

    delete_server(&mut transaction, *server_id)
        .await
        .context("Failed to delete existing server from database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new server.")?;

    // Notify websocket server about the deleted server
    // WSTODO
    // websocket_server.do_send(messages::DeleteServer::new(*server_id));

    Ok(HttpResponse::Ok().finish())
}
