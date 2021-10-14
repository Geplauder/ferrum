use std::convert::{TryFrom, TryInto};

use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::users::{
    models::{NewUser, UserEmail, UserName, UserPassword},
    queries::insert_user,
};
pub use ferrum_shared::error_chain_fmt;
use sqlx::PgPool;

///
/// Contains the request body for registering users.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    name: String,
    email: String,
    password: String,
}

///
/// Try to convert [`BodyData`] into a validated instance of [`NewUser`].
///
impl TryFrom<BodyData> for NewUser {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = UserName::parse(value.name)?;
        let email = UserEmail::parse(value.email)?;
        let password = UserPassword::parse(value.password)?;

        Ok(Self {
            name,
            email,
            password,
        })
    }
}

///
/// Possible errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum RegisterError {
    /// Invalid data was supplied in the request.
    #[error("{0}")]
    ValidationError(String),
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for RegisterError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            RegisterError::ValidationError(_) => StatusCode::BAD_REQUEST,
            RegisterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Register new user", skip(body, pool), fields(user_name = %body.name, user_email = %body.email))]
pub async fn register(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, RegisterError> {
    // Validate the request body
    let new_user: NewUser = body.0.try_into().map_err(RegisterError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a postgres connection from the pool")?;

    insert_user(&mut transaction, &new_user)
        .await
        .context("Failed to insert new user into database")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new user.")?;

    Ok(HttpResponse::Ok().finish())
}
