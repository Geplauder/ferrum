use crate::telemetry::spawn_blocking_with_tracing;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use ferrum_db::users::queries::get_user_with_email;
pub use ferrum_shared::error_chain_fmt;
use ferrum_shared::jwt::Jwt;
use sqlx::{types::Uuid, PgPool};

///
/// Contains the request body for logging in users.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    email: String,
    password: String,
}

struct LoginUser {
    email: String,
    password: String,
}

///
/// Try to convert [`BodyData`] into a validated instance of [`LoginUser`].
///
impl From<BodyData> for LoginUser {
    fn from(value: BodyData) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}

///
/// Possibles errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum LoginError {
    /// Login failed due to wrong email or password
    #[error("Login failed")]
    LoginFailed(#[source] anyhow::Error),
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            LoginError::LoginFailed(_) => StatusCode::UNAUTHORIZED,
            LoginError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Login user", skip(body, pool), fields(user_email = %body.email))]
pub async fn login(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    jwt: web::Data<Jwt>,
) -> Result<HttpResponse, LoginError> {
    let login_user: LoginUser = body.0.into();

    let (user_id, user_email) = validate_credentials(login_user, &pool).await?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    let token = jwt.encode(user_id, user_email);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "token": token,
    })))
}

#[tracing::instrument(name = "Validate credentials", skip(login_user, pool))]
async fn validate_credentials(
    login_user: LoginUser,
    pool: &PgPool,
) -> Result<(Uuid, String), LoginError> {
    // Try to get a user with the supplied email
    let stored_user = get_user_with_email(login_user.email.as_ref(), pool)
        .await
        .context("Failed to retrieve stored user.")?;

    // If no user was found, return a logging failed error
    let user = match stored_user {
        Some(value) => value,
        None => return Err(LoginError::LoginFailed(anyhow::anyhow!(""))),
    };

    let user_password = user.password.clone();

    // Check if the supplied password matches the one stored for this user
    spawn_blocking_with_tracing(move || verify_password_hash(user_password, login_user.password))
        .await
        .context("Failed to spawn blocking task.")
        .map_err(LoginError::UnexpectedError)??;

    Ok((user.id, user.email))
}

#[tracing::instrument(
    name = "Verify credentials",
    skip(expected_password_hash, given_password)
)]
fn verify_password_hash(
    expected_password_hash: String,
    given_password: String,
) -> Result<(), LoginError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse password hash.")
        .map_err(LoginError::UnexpectedError)?;

    // Check the given password against the stored one and return a login failed error if they do not match
    Argon2::default()
        .verify_password(given_password.as_bytes(), &expected_password_hash)
        .context("Invalid password.")
        .map_err(LoginError::LoginFailed)
}
