use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::channels::Channel, error_chain_fmt, jwt::AuthorizationService,
    utilities::is_user_on_server,
};

#[derive(thiserror::Error)]
pub enum GetChannelsError {
    #[error("Forbidden")]
    ForbiddenError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetChannelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetChannelsError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            GetChannelsError::ForbiddenError => StatusCode::FORBIDDEN,
            GetChannelsError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get server channels", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn get_channels(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, GetChannelsError> {
    let is_user_on_server = is_user_on_server(&pool, auth.claims.id, *server_id)
        .await
        .context("Failed to check if user is on server")?;

    if is_user_on_server == false {
        return Err(GetChannelsError::ForbiddenError);
    }

    let server_channels = get_server_channels(*server_id, &pool).await?;

    Ok(HttpResponse::Ok().json(server_channels))
}

#[tracing::instrument(name = "Get server channels", skip(server_id, pool))]
async fn get_server_channels(
    server_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<Channel>, GetChannelsError> {
    let channels = sqlx::query_as!(
        Channel,
        r#"
        SELECT *
        FROM channels
        WHERE channels.server_id = $1
        "#,
        server_id
    )
    .fetch_all(pool)
    .await
    .context("Failed to retrieve server channels.")?;

    Ok(channels)
}
