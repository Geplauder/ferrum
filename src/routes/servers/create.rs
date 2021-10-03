use std::convert::{TryFrom, TryInto};

use actix::Addr;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::servers::models::{NewServer, ServerModel, ServerName};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error_chain_fmt,
    jwt::AuthorizationService,
    websocket::{messages, Server as WebSocketServer},
};

#[derive(serde::Deserialize)]
pub struct BodyData {
    name: String,
}

impl TryFrom<BodyData> for NewServer {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = ServerName::parse(value.name)?;

        Ok(Self { name })
    }
}

#[derive(thiserror::Error)]
pub enum CreateError {
    #[error("{0}")]
    ValidationError(String),
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
    let new_server: NewServer = body.0.try_into().map_err(CreateError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from pool")?;

    let server = insert_server(&mut transaction, &new_server, auth.claims.id)
        .await
        .context("Failed to insert new server in the database.")?;

    add_default_channel_to_server(&mut transaction, server.id)
        .await
        .context("Failed to insert default server channel to the database.")?;

    add_user_to_server(&mut transaction, auth.claims.id, server.id)
        .await
        .context("Failed to insert new users_servers entry to the database.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new server.")?;

    websocket_server.do_send(messages::NewServer::new(auth.claims.id, server.id));

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving a new server to the database",
    skip(transaction, new_server)
)]
async fn insert_server(
    transaction: &mut Transaction<'_, Postgres>,
    new_server: &NewServer,
    user_id: Uuid,
) -> Result<ServerModel, sqlx::Error> {
    let server = sqlx::query_as!(
        ServerModel,
        r#"
        INSERT INTO servers (id, name, owner_id)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        Uuid::new_v4(),
        new_server.name.as_ref(),
        user_id,
    )
    .fetch_one(transaction)
    .await?;

    Ok(server)
}

#[tracing::instrument(
    name = "Saving a new default server channel to the database",
    skip(transaction, server_id)
)]
async fn add_default_channel_to_server(
    transaction: &mut Transaction<'_, Postgres>,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO channels (id, server_id, name) VALUES ($1, $2, $3)
        "#,
        Uuid::new_v4(),
        server_id,
        "general",
    )
    .execute(transaction)
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving a new users_servers entry to the database",
    skip(transaction, user_id, server_id)
)]
async fn add_user_to_server(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO users_servers (id, user_id, server_id) VALUES ($1, $2, $3)
        "#,
        id,
        user_id,
        server_id,
    )
    .execute(transaction)
    .await?;

    Ok(())
}
