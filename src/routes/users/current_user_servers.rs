use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::queries::get_servers_for_user;
use ferrum_shared::servers::ServerResponse;
use sqlx::PgPool;

use crate::{error_chain_fmt, jwt::AuthorizationService};

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

impl ResponseError for CurrentUserServersError {}

#[tracing::instrument(name = "Get current user servers", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn current_user_servers(
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CurrentUserServersError> {
    let user_servers: Vec<ServerResponse> = get_servers_for_user(auth.claims.id, &pool)
        .await
        .context("Failed to retrieve server users")?
        .iter()
        .map(|x| x.clone().into())
        .collect();

    Ok(HttpResponse::Ok().json(user_servers))
}
