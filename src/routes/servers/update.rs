use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::{
    models::{ServerName, UpdateServer},
    queries::{is_user_owner_of_server, update_server_name},
};
use ferrum_shared::{broker::BrokerEvent, error_chain_fmt, jwt::AuthorizationService};
use sqlx::PgPool;
use uuid::Uuid;

use crate::broker::{Broker, PublishBrokerEvent};

///
/// Contains the request body for updating a server.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    name: Option<String>,
}

///
/// Try to convert [`BodyData`] into validated instance of [`UpdateServer`].
///
impl TryFrom<BodyData> for UpdateServer {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = if let Some(value) = value.name {
            Some(ServerName::parse(value)?)
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
pub enum ServerUpdateError {
    /// Invalid data was supplied in the request.
    #[error("{0}")]
    ValidationError(String),
    /// User has no permission to update a server.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ServerUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ServerUpdateError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            ServerUpdateError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ServerUpdateError::ForbiddenError => StatusCode::FORBIDDEN,
            ServerUpdateError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Update existing server", skip(body, pool, broker, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn update(
    server_id: web::Path<Uuid>,
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    broker: web::Data<Addr<Broker>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, ServerUpdateError> {
    // Validate the request body
    let update_server: UpdateServer = body
        .0
        .try_into()
        .map_err(ServerUpdateError::ValidationError)?;

    // Check if the user is the owner of this server, return forbidden error if not
    let is_user_owner = is_user_owner_of_server(&pool, *server_id, auth.claims.id)
        .await
        .context("Failed to check if user is owner of the server.")?;

    if is_user_owner == false {
        return Err(ServerUpdateError::ForbiddenError);
    }

    // Update name, if it is inlcuded in the request body
    if let Some(name) = &update_server.name {
        update_server_name(&pool, *server_id, name)
            .await
            .context("Failed to update server name")?
    }

    // WSTODO
    broker.do_send(PublishBrokerEvent {
        broker_event: BrokerEvent::UpdateServer {
            server_id: *server_id,
        },
    });

    Ok(HttpResponse::Ok().finish())
}
