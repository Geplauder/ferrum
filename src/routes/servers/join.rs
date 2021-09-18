use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum JoinError {
    #[error("Server id has an invalid format!")]
    ValidationError(#[from] uuid::Error),
    #[error("User is already member of that server")]
    AlreadyJoinedError,
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

pub async fn join(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, JoinError> {
    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from the pool")?;

    insert_user_server(&mut transaction, auth.claims.id, *server_id).await?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new users_servers entry.")?;

    Ok(HttpResponse::Ok().finish())
}

async fn insert_user_server(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), JoinError> {
    let id = Uuid::new_v4();

    match sqlx::query!(
        r#"
        INSERT INTO users_servers (id, user_id, server_id)
        VALUES ($1, $2, $3)
        "#,
        id,
        user_id,
        server_id,
    )
    .execute(transaction)
    .await
    {
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
    .map_err(JoinError::UnexpectedError)
}
