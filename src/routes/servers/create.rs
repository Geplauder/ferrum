use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::{
    models::{NewServer, ServerName},
    queries::{add_default_channel_to_server, add_user_to_server, insert_server},
};
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::jwt::AuthorizationService;
use ferrum_websocket::{messages, WebSocketServer};
use sqlx::PgPool;

///
/// Contains the request body for creating servers.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    name: String,
}

///
/// Try to convert [`BodyData`] into a validated instance of [`NewServer`].
///
impl TryFrom<BodyData> for NewServer {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = ServerName::parse(value.name)?;

        Ok(Self { name })
    }
}

///
/// Possibles errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum CreateError {
    /// Invalid data was supplied in the request.
    #[error("{0}")]
    ValidationError(String),
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for CreateError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            CreateError::ValidationError(_) => StatusCode::BAD_REQUEST,
            CreateError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Create a new server", skip(body, pool, websocket_server, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email, server_name = %body.name))]
pub async fn create(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    websocket_server: web::Data<Addr<WebSocketServer>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, CreateError> {
    // Validate the request body
    let new_server: NewServer = body.0.try_into().map_err(CreateError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool")?;

    // Create the server
    let server = insert_server(&mut transaction, &new_server, auth.claims.id)
        .await
        .context("Failed to insert new server in the database.")?;

    // Add a default channel
    add_default_channel_to_server(&mut transaction, server.id)
        .await
        .context("Failed to insert default server channel to the database.")?;

    // Add the owner to the server
    add_user_to_server(&mut transaction, auth.claims.id, server.id)
        .await
        .context("Failed to insert new users_servers entry to the database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new server.")?;

    // Notify websocket server about the new server
    websocket_server.do_send(messages::NewServer::new(auth.claims.id, server.id));

    Ok(HttpResponse::Ok().finish())
}
