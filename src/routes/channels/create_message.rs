use std::convert::{TryFrom, TryInto};

use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::messages::{MessageContent, NewMessage},
    error_chain_fmt,
    jwt::AuthorizationService,
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    content: String,
}

impl TryFrom<BodyData> for NewMessage {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let content = MessageContent::parse(value.content)?;

        Ok(Self { content })
    }
}

#[derive(thiserror::Error)]
pub enum CreateMessageError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Unauthorized")]
    UnauthorizedError(#[from] sqlx::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CreateMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for CreateMessageError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            CreateMessageError::ValidationError(_) => StatusCode::BAD_REQUEST,
            CreateMessageError::UnauthorizedError(_) => StatusCode::UNAUTHORIZED,
            CreateMessageError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Create a new channel message", skip(body, pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn create_message(
    channel_id: web::Path<Uuid>,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CreateMessageError> {
    let new_message: NewMessage = body
        .0
        .try_into()
        .map_err(CreateMessageError::ValidationError)?;

    // TODO: Check if user has access to that channel
    does_user_have_access_to_channel(&pool, *channel_id, auth.claims.id)
        .await
        .map_err(CreateMessageError::UnauthorizedError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool.")?;

    insert_message(&mut transaction, &new_message, *channel_id, auth.claims.id)
        .await
        .context("Failed to insert a new channel message to the database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new channel message.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving a new channel message to the database",
    skip(transaction, new_message, channel_id, user_id)
)]
async fn insert_message(
    transaction: &mut Transaction<'_, Postgres>,
    new_message: &NewMessage,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO messages (id, channel_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        "#,
        id,
        channel_id,
        user_id,
        new_message.content.as_ref(),
    )
    .execute(transaction)
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Check if user has access to a channel",
    skip(pool, channel_id, user_id)
)]
async fn does_user_have_access_to_channel(
    pool: &PgPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        WITH server_query AS (
            SELECT servers.id as server_id
            FROM servers
            INNER JOIN channels ON channels.server_id = servers.id
            WHERE channels.id = $1 LIMIT 1
        )
        SELECT users_servers.*
        FROM users_servers
        WHERE users_servers.user_id = $2 AND users_servers.server_id IN (SELECT server_id FROM server_query)
        "#,
        channel_id,
        user_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(())
}
