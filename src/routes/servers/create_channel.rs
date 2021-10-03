use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{
    channels::{
        models::{ChannelName, NewChannel},
        queries::insert_channel,
    },
    servers::queries::is_user_owner_of_server,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error_chain_fmt,
    jwt::AuthorizationService,
    websocket::{messages, Server},
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
    #[error("Forbidden")]
    ForbiddenError,
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
            CreateChannelError::ForbiddenError => StatusCode::FORBIDDEN,
            CreateChannelError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Create a new server channel", skip(body, pool, websocket_server, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email, channel_name = %body.name))]
pub async fn create_channel(
    server_id: web::Path<Uuid>,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    websocket_server: web::Data<Addr<Server>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CreateChannelError> {
    let new_channel: NewChannel = body
        .0
        .try_into()
        .map_err(CreateChannelError::ValidationError)?;

    let is_user_owner = is_user_owner_of_server(&pool, *server_id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(CreateChannelError::ForbiddenError);
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool.")?;

    let channel = insert_channel(&mut transaction, &new_channel, *server_id)
        .await
        .context("Failed to insert new server channel to the database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new server channel.")?;

    websocket_server.do_send(messages::NewChannel::new(channel.into()));

    Ok(HttpResponse::Ok().finish())
}
