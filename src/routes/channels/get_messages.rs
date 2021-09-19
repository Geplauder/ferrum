use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::messages::Message, error_chain_fmt, jwt::AuthorizationService,
    utilities::does_user_have_access_to_channel,
};

#[derive(thiserror::Error)]
pub enum GetMessagesError {
    #[error("Unauthorized")]
    UnauthorizedError(#[from] sqlx::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetMessagesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetMessagesError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            GetMessagesError::UnauthorizedError(_) => StatusCode::UNAUTHORIZED,
            GetMessagesError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get messages for channel", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn get_messages(
    channel_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, GetMessagesError> {
    does_user_have_access_to_channel(&pool, *channel_id, auth.claims.id)
        .await
        .map_err(GetMessagesError::UnauthorizedError)?;

    let channel_messages = get_channel_messages(*channel_id, &pool).await?;

    Ok(HttpResponse::Ok().json(channel_messages))
}

#[tracing::instrument(name = "Get messages from channel", skip(channel_id, pool))]
async fn get_channel_messages(
    channel_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<Message>, GetMessagesError> {
    let messages = sqlx::query_as!(
        Message,
        r#"
        SELECT *
        FROM messages
        WHERE messages.channel_id = $1
        "#,
        channel_id,
    )
    .fetch_all(pool)
    .await
    .context("Failed to retrieve channel messages.")?;

    Ok(messages)
}
