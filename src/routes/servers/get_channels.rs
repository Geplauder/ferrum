use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::{channels::queries::get_channels_for_server, users::queries::is_user_on_server};
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::{channels::ChannelResponse, jwt::AuthorizationService};
use sqlx::PgPool;
use uuid::Uuid;

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum GetChannelsError {
    /// User has no permissions to access this server.
    #[error("Forbidden")]
    ForbiddenError,
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for GetChannelsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for GetChannelsError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            GetChannelsError::ForbiddenError => StatusCode::FORBIDDEN,
            GetChannelsError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Get server channels", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn get_channels(
    server_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, GetChannelsError> {
    // Check if the authenticated user is on the server, return forbidden error if not
    let is_user_on_server = is_user_on_server(&pool, auth.claims.id, *server_id)
        .await
        .context("Failed to check if user is on server")?;

    if is_user_on_server == false {
        return Err(GetChannelsError::ForbiddenError);
    }

    // Get all channels for this server and transform them into proper responses
    let server_channels: Vec<ChannelResponse> = get_channels_for_server(*server_id, &pool)
        .await
        .context("Failed to retrieve server channels.")?
        .iter()
        .map(|x| x.clone().into())
        .collect();

    Ok(HttpResponse::Ok().json(server_channels))
}
