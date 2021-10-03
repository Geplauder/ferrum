use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{
    messages::queries::get_messages_for_channel, users::queries::does_user_have_access_to_channel,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error_chain_fmt, jwt::AuthorizationService};

#[derive(thiserror::Error)]
pub enum GetMessagesError {
    #[error("Forbidden")]
    ForbiddenError(#[from] sqlx::Error),
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
            GetMessagesError::ForbiddenError(_) => StatusCode::FORBIDDEN,
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
        .map_err(GetMessagesError::ForbiddenError)?;

    let _channel_messages = get_messages_for_channel(*channel_id, &pool)
        .await
        .context("Failed to retrieve channel messages.")?;

    todo!("Return MessageResponse");

    // Ok(HttpResponse::Ok().json(channel_messages))
}
