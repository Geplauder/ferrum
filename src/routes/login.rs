use std::convert::{TryFrom, TryInto};

use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use ferrum_db::users::queries::get_user_with_email;
use sqlx::{types::Uuid, PgPool};

use crate::{error_chain_fmt, jwt::Jwt, telemetry::spawn_blocking_with_tracing};

#[derive(serde::Deserialize)]
pub struct BodyData {
    email: String,
    password: String,
}

struct LoginUser {
    email: String,
    password: String,
}

impl TryFrom<BodyData> for LoginUser {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        Ok(Self {
            email: value.email,
            password: value.password,
        })
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Login failed")]
    LoginFailed(#[source] anyhow::Error),
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
            LoginError::ValidationError(_) => StatusCode::BAD_REQUEST,
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
    let login_user: LoginUser = body.0.try_into().map_err(LoginError::ValidationError)?;

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
    let stored_user = get_user_with_email(login_user.email.as_ref(), pool)
        .await
        .map_err(LoginError::UnexpectedError)?;

    let user = match stored_user {
        Some(value) => value,
        None => return Err(LoginError::LoginFailed(anyhow::anyhow!(""))),
    };

    let user_password = user.password.clone();

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

    Argon2::default()
        .verify_password(given_password.as_bytes(), &expected_password_hash)
        .context("Invalid password.")
        .map_err(LoginError::LoginFailed)
}
