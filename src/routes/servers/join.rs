use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::queries::add_user_to_server;
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::jwt::AuthorizationService;
use ferrum_websocket::messages::BrokerEvent;
use sqlx::PgPool;
use uuid::Uuid;

use crate::broker::{Broker, PublishBrokerEvent};

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum JoinError {
    /// Invalid data was supplied in the request.
    #[error("Server id has an invalid format!")]
    ValidationError(#[from] uuid::Error),
    /// User is already member of this server.
    #[error("User is already member of that server")]
    AlreadyJoinedError,
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for JoinError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            JoinError::ValidationError(_) => StatusCode::BAD_REQUEST,
            JoinError::AlreadyJoinedError => StatusCode::NO_CONTENT,
            JoinError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Join server", skip(pool, auth, broker), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn join(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    broker: web::Data<Addr<Broker>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, JoinError> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from the pool")?;

    // Try to add the user to the server, return already joined error if the user is already on it
    match add_user_to_server(&mut transaction, auth.claims.id, *server_id).await {
        Ok(_) => Ok(()),
        Err(error) => {
            if error.as_database_error().unwrap().code().unwrap() == "23505" {
                return Err(JoinError::AlreadyJoinedError);
            } else {
                Err(error)
            }
        }
    }
    .context("Failed to insert new users_servers entry in the database")
    .map_err(JoinError::UnexpectedError)?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new users_servers entry.")?;

    // Notify the websocket server about the new user
    // TODO: Handle everything in one websocket message
    broker.do_send(PublishBrokerEvent {
        broker_event: BrokerEvent::NewServer {
            user_id: auth.claims.id,
            server_id: *server_id,
        },
    });

    broker.do_send(PublishBrokerEvent {
        broker_event: BrokerEvent::NewUser {
            user_id: auth.claims.id,
            server_id: *server_id,
        },
    });

    Ok(HttpResponse::Ok().finish())
}
