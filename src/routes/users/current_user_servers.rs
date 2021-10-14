use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::queries::get_servers_for_user;
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::{jwt::AuthorizationService, servers::ServerResponse};
use sqlx::PgPool;

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum CurrentUserServersError {
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CurrentUserServersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for CurrentUserServersError {}

#[tracing::instrument(name = "Get current user servers", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn current_user_servers(
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CurrentUserServersError> {
    // Get servers for the currently authenticated user and return them.
    let user_servers: Vec<ServerResponse> = get_servers_for_user(auth.claims.id, &pool)
        .await
        .context("Failed to retrieve server users")?
        .iter()
        .map(|x| x.clone().into())
        .collect();

    Ok(HttpResponse::Ok().json(user_servers))
}
