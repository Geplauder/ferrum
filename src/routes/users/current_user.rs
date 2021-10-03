use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::users::queries::get_user_with_id;
use sqlx::PgPool;

use crate::{error_chain_fmt, jwt::AuthorizationService};

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

impl ResponseError for UsersError {}

#[tracing::instrument(name = "Get current user information", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn current_user(
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, UsersError> {
    let user_data = get_user_with_id(auth.claims.id, &pool)
        .await
        .context("Failed to retrieve stored user.")?;

    Ok(HttpResponse::Ok().json(user_data))
}
