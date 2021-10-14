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
    messages::{self, WebSocketMessage},
    WebSocketServer,
};
use sqlx::PgPool;
use uuid::Uuid;

///
/// Contains the request body for creating messages.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    content: String,
}

///
/// Try to convert [`BodyData`] into a validated instance of [`NewMessage`].
///
impl TryFrom<BodyData> for NewMessage {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let content = MessageContent::parse(value.content)?;

        Ok(Self { content })
    }
}

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum CreateMessageError {
    /// Invalid data was supplied in the request.
    #[error("{0}")]
    ValidationError(String),
    /// User has no permissions to send messages in this channel.
    #[error("Forbidden")]
    ForbiddenError(#[from] sqlx::Error),
    /// An unexpected error has occoured while processing the request.
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
    // Validate the request body
    let new_message: NewMessage = body
        .0
        .try_into()
        .map_err(CreateMessageError::ValidationError)?;

    // Check if the authenticated user has access to the channel, return forbidden error if not
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

    let user = get_user_with_id(auth.claims.id, &pool)
        .await
        .context("Failed to get user model")?;

    // Notify the websocket server about the new message
    server.do_send(messages::SendMessageToChannel::new(
        *channel_id,
        WebSocketMessage::NewMessage {
            message: message.to_response(user),
        },
    ));

    Ok(HttpResponse::Ok().finish())
}
