use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{
    messages::{
        models::{MessageContent, NewMessage},
        queries::insert_message,
    },
    users::queries::{does_user_have_access_to_channel, get_user_with_id},
};
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::jwt::AuthorizationService;
use ferrum_websocket::{
    messages::{self, WebSocketMessageType},
    WebSocketServer,
};
use sqlx::PgPool;
use uuid::Uuid;

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
    server: web::Data<Addr<WebSocketServer>>,
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

    let user = get_user_with_id(auth.claims.id, &pool)
        .await
        .context("Failed to get user model")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new channel message.")?;

    server.do_send(messages::SendMessageToChannel::new(
        *channel_id,
        WebSocketMessageType::NewMessage {
            message: message.to_response(user),
        },
    ));

    Ok(HttpResponse::Ok().finish())
}
