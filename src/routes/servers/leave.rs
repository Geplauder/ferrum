use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::queries::{get_server_with_id, remove_user_from_server};
use ferrum_shared::{error_chain_fmt, jwt::AuthorizationService};
use ferrum_websocket::WebSocketServer;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error)]
pub enum LeaveError {
    #[error("User is not a member of this server")]
    NotOnServerError,
    #[error("User is owner of this server")]
    UserIsOwnerError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LeaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LeaveError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            LeaveError::NotOnServerError => StatusCode::BAD_REQUEST,
            LeaveError::UserIsOwnerError => StatusCode::BAD_REQUEST,
            LeaveError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Leave server", skip(pool, auth, _websocket_server), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn leave(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    _websocket_server: web::Data<Addr<WebSocketServer>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, LeaveError> {
    let server = get_server_with_id(*server_id, &pool)
        .await
        .context("Could not fetch server")?;

    if auth.claims.id == server.owner_id {
        return Err(LeaveError::UserIsOwnerError);
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from the pool")?;

    let was_user_on_server = remove_user_from_server(&mut transaction, auth.claims.id, *server_id)
        .await
        .context("Failed to remove users_servers entry from the database")?;

    if was_user_on_server == false {
        return Err(LeaveError::NotOnServerError);
    }

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction.")?;

    Ok(HttpResponse::Ok().finish())
}
