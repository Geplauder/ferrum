use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{servers::queries::get_server_with_id, users::queries::is_user_on_server};
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::{jwt::AuthorizationService, servers::ServerResponse};
use sqlx::PgPool;
use uuid::Uuid;

///
/// Possibles errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum GetServerError {
    /// User has no permissions to access this server.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occoured while processing the request.
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
    // Check if the authenticated user is on the server, return forbidden error if not
    let is_user_on_server = is_user_on_server(&pool, auth.claims.id, *server_id)
        .await
        .context("Failed to check if user is on server")?;

    if is_user_on_server == false {
        return Err(GetServerError::ForbiddenError);
    }

    // Get the specified server and transform it into a proper response
    let server: ServerResponse = get_server_with_id(*server_id, &pool)
        .await
        .context("Error while fetching server")?
        .into();

    Ok(HttpResponse::Ok().json(server))
}
