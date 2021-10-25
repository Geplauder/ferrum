use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{
    channels::queries::delete_channel,
    servers::queries::{get_server_for_channel_id, is_user_owner_of_server},
};
use ferrum_shared::{broker::BrokerEvent, error_chain_fmt, jwt::AuthorizationService};
use sqlx::PgPool;
use uuid::Uuid;

use crate::broker::{Broker, PublishBrokerEvent};

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum DeleteChannelError {
    /// User has no permission to delete this channel.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for DeleteChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for DeleteChannelError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            DeleteChannelError::ForbiddenError => StatusCode::FORBIDDEN,
            DeleteChannelError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument("Delete channel", skip(pool, broker, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn delete(
    channel_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    broker: web::Data<Addr<Broker>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, DeleteChannelError> {
    // Get the server that the channel belongs to
    let server = get_server_for_channel_id(*channel_id, &pool)
        .await
        .context("Failed to get server for channel")?;

    // Check if the user is the owner of this server, return forbidden error if not
    let is_user_owner = is_user_owner_of_server(&pool, server.id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(DeleteChannelError::ForbiddenError);
    }

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool.")?;

    delete_channel(&mut transaction, *channel_id)
        .await
        .context("Failed to delete existing channel from database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to delete existing channel.")?;

    // Notify websocket about the deleted channel
    broker.do_send(PublishBrokerEvent {
        broker_event: BrokerEvent::DeleteChannel {
            server_id: server.id,
            channel_id: *channel_id,
        },
    });

    Ok(HttpResponse::Ok().finish())
}
