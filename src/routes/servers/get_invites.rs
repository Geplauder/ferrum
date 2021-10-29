use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{
    server_invites::queries::get_server_invites_for_server,
    servers::queries::is_user_owner_of_server,
};
use ferrum_shared::{
    error_chain_fmt, jwt::AuthorizationService, server_invites::ServerInviteResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum GetInvitesError {
    /// User has no permissions to get invites for this server.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetInvitesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetInvitesError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            GetInvitesError::ForbiddenError => StatusCode::FORBIDDEN,
            GetInvitesError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get server invites", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn get_invites(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, GetInvitesError> {
    // Check if the user is the owner of this server, return forbidden error if not
    let is_user_owner = is_user_owner_of_server(&pool, *server_id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(GetInvitesError::ForbiddenError);
    }

    // Get all invites for this server, transform them into responses and return them in the response
    let server_invites: Vec<ServerInviteResponse> =
        get_server_invites_for_server(*server_id, &pool)
            .await
            .context("Failed to get server invites for server.")?
            .iter()
            .map(|x| x.clone().into())
            .collect();

    Ok(HttpResponse::Ok().json(server_invites))
}
