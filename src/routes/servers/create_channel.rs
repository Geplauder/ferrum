use std::convert::{TryFrom, TryInto};

use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::channels::{ChannelName, NewChannel},
    error_chain_fmt,
    jwt::AuthorizationService,
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    name: String,
}

impl TryFrom<BodyData> for NewChannel {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = ChannelName::parse(value.name)?;

        Ok(Self { name })
    }
}

#[derive(thiserror::Error)]
pub enum CreateChannelError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Unauthorized")]
    UnauthorizedError,
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CreateChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for CreateChannelError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            CreateChannelError::ValidationError(_) => StatusCode::BAD_REQUEST,
            CreateChannelError::UnauthorizedError => StatusCode::UNAUTHORIZED,
            CreateChannelError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Create a new server channel", skip(body, pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email, channel_name = %body.name))]
pub async fn create_channel(
    server_id: web::Path<Uuid>,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CreateChannelError> {
    let new_channel: NewChannel = body
        .0
        .try_into()
        .map_err(CreateChannelError::ValidationError)?;

    let is_user_owner = check_if_user_is_owner(&pool, *server_id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(CreateChannelError::UnauthorizedError);
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool.")?;

    insert_channel(&mut transaction, &new_channel, *server_id)
        .await
        .context("Failed to insert new server channel to the database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new server channel.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Check if user is owner of the server",
    skip(pool, server_id, user_id)
)]
async fn check_if_user_is_owner(
    pool: &PgPool,
    server_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!("SELECT owner_id FROM servers WHERE id = $1", server_id)
        .fetch_one(pool)
        .await?;

    Ok(row.owner_id == user_id)
}

#[tracing::instrument(
    name = "Saving a new server channel to the database",
    skip(transaction, new_channel, server_id)
)]
async fn insert_channel(
    transaction: &mut Transaction<'_, Postgres>,
    new_channel: &NewChannel,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO channels (id, server_id, name)
        VALUES ($1, $2, $3)
        "#,
        id,
        server_id,
        new_channel.name.as_ref(),
    )
    .execute(transaction)
    .await?;

    Ok(())
}
