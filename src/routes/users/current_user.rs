use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use ferrum_db::users::queries::get_user_with_id;
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::{jwt::AuthorizationService, users::UserResponse};
use sqlx::PgPool;

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum UsersError {
    /// No user for the id was found
    #[error("Unauthorized")]
    UnauthorizedError,
}

impl std::fmt::Debug for UsersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UsersError {
    fn status_code(&self) -> actix_http::StatusCode {
        StatusCode::UNAUTHORIZED
    }
}

#[tracing::instrument(name = "Get current user information", skip(pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn current_user(
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, UsersError> {
    // Get the user data for the currently authenticated user and return it, if found.
    let user_data: UserResponse = get_user_with_id(auth.claims.id, &pool)
        .await
        .map_err(|_| UsersError::UnauthorizedError)?
        .into();

    Ok(HttpResponse::Ok().json(user_data))
}
