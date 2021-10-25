use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{
    channels::{
        models::{ChannelName, UpdateChannel},
        queries::update_channel_name,
    },
    servers::queries::{get_server_for_channel_id, is_user_owner_of_server},
};
use ferrum_shared::{broker::BrokerEvent, error_chain_fmt, jwt::AuthorizationService};
use sqlx::PgPool;
use uuid::Uuid;

use crate::broker::{Broker, PublishBrokerEvent};

///
/// Contains the request body for updating a channel.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    name: Option<String>,
}

///
/// Try to convert [`BodyData`] into validated instance of [`UpdateChannel`].
///
impl TryFrom<BodyData> for UpdateChannel {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = if let Some(value) = value.name {
            Some(ChannelName::parse(value)?)
        } else {
            None
        };

        Ok(Self { name })
    }
}

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum ChannelUpdateError {
    /// Invalid data was supplied in the request.
    #[error("{0}")]
    ValidationError(String),
    /// User has no permission to update a channel.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ChannelUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ChannelUpdateError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            ChannelUpdateError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ChannelUpdateError::ForbiddenError => StatusCode::FORBIDDEN,
            ChannelUpdateError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Update existing channel", skip(body, pool, broker, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn update(
    channel_id: web::Path<Uuid>,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    broker: web::Data<Addr<Broker>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, ChannelUpdateError> {
    // Validate the request body
    let update_channel: UpdateChannel = body
        .0
        .try_into()
        .map_err(ChannelUpdateError::ValidationError)?;

    // Get the server that the channel belongs to
    let server = get_server_for_channel_id(*channel_id, &pool)
        .await
        .context("Failed to get server for channel")?;

    // Check if the user is the owner of this server, return forbidden error if not
    let is_user_owner = is_user_owner_of_server(&pool, server.id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(ChannelUpdateError::ForbiddenError);
    }

    // Update name, if it is inlcuded in the request body
    if let Some(name) = &update_channel.name {
        update_channel_name(&pool, *channel_id, name)
            .await
            .context("Failed to update channel name")?
    }

    // Notify websocket about the deleted channel
    broker.do_send(PublishBrokerEvent {
        broker_event: BrokerEvent::UpdateChannel {
            channel_id: *channel_id,
        },
    });

    Ok(HttpResponse::Ok().finish())
}
