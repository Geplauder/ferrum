use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{
        messages::{Message, MessageContent, MessageResponse, NewMessage},
        users::User,
    },
    error_chain_fmt,
    jwt::AuthorizationService,
    utilities::does_user_have_access_to_channel,
    websocket::{
        messages::{SendMessageToChannel, WebSocketMessage},
        Server,
    },
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
    #[error("Forbidden")]
    ForbiddenError(#[from] sqlx::Error),
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
            CreateMessageError::ForbiddenError(_) => StatusCode::FORBIDDEN,
            CreateMessageError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Create a new channel message", skip(body, pool, auth, server), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn create_message(
    channel_id: web::Path<Uuid>,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    server: web::Data<Addr<Server>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CreateMessageError> {
    let new_message: NewMessage = body
        .0
        .try_into()
        .map_err(CreateMessageError::ValidationError)?;

    does_user_have_access_to_channel(&pool, *channel_id, auth.claims.id)
        .await
        .map_err(CreateMessageError::ForbiddenError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool.")?;

    let message = insert_message(
        &mut transaction,
        &pool,
        &new_message,
        *channel_id,
        auth.claims.id,
    )
    .await
    .context("Failed to insert a new channel message to the database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new channel message.")?;

    server.do_send(SendMessageToChannel::new(
        *channel_id,
        WebSocketMessage::NewMessage { message },
    ));

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving a new channel message to the database",
    skip(transaction, new_message, channel_id, user_id)
)]
async fn insert_message(
    transaction: &mut Transaction<'_, Postgres>,
    pool: &PgPool,
    new_message: &NewMessage,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<MessageResponse, sqlx::Error> {
    let id = Uuid::new_v4();

    let message = sqlx::query_as!(
        Message,
        r#"
        INSERT INTO messages (id, channel_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        id,
        channel_id,
        user_id,
        new_message.content.as_ref(),
    )
    .fetch_one(transaction)
    .await?;

    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, created_at, updated_at
        FROM users
        WHERE users.id = $1
        "#,
        message.user_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(MessageResponse::new(&message, &user))
}
