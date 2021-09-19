use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::channels::Channel, error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum GetChannelsError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetChannelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetChannelsError {}

pub async fn get_channels(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    _auth: AuthorizationService,
) -> Result<HttpResponse, GetChannelsError> {
    // TODO: Check if authenticated user is on server
    let server_channels = get_server_channels(*server_id, &pool).await?;

    Ok(HttpResponse::Ok().json(server_channels))
}

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
