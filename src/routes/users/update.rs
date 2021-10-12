use std::convert::{TryFrom, TryInto};

use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use ferrum_db::users::{
    models::{verify_password_hash, UpdateUser, UserEmail, UserName, UserPassword},
    queries::{get_user_with_id, update_user_email, update_user_name, update_user_password},
};
use ferrum_shared::{error_chain_fmt, jwt::AuthorizationService};
use sqlx::PgPool;

use crate::telemetry::spawn_blocking_with_tracing;

///
/// Contains the request body for updating the current user.
///
#[derive(serde::Deserialize)]
pub struct BodyData {
    name: Option<String>,
    email: Option<String>,
    password: Option<String>,
    current_password: String,
}

///
/// Try to convert [`BodyData`] into a validated instance of [`UpdateUser`].
///
impl TryFrom<BodyData> for UpdateUser {
    type Error = String;

    fn try_from(value: BodyData) -> Result<Self, Self::Error> {
        let name = if let Some(value) = value.name {
            Some(UserName::parse(value)?)
        } else {
            None
        };

        let email = if let Some(value) = value.email {
            Some(UserEmail::parse(value)?)
        } else {
            None
        };

        let password = if let Some(value) = value.password {
            Some(UserPassword::parse(value)?)
        } else {
            None
        };

        Ok(Self {
            name,
            email,
            password,
            current_password: value.current_password,
        })
    }
}

///
/// Possibles errors that can occur on this route.
///
#[derive(thiserror::Error)]
pub enum UserUpdateError {
    /// User provided the wrong current password
    #[error("Wrong password")]
    WrongPasswordError,
    /// Invalid data was supplied in the request.
    #[error("{0}")]
    ValidationError(String),
    /// An unexpected error has occoured while processing the request.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for UserUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for UserUpdateError {
    fn status_code(&self) -> actix_http::StatusCode {
        match *self {
            UserUpdateError::WrongPasswordError => StatusCode::FORBIDDEN,
            UserUpdateError::ValidationError(_) => StatusCode::BAD_REQUEST,
            UserUpdateError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(name = "Update existing user", skip(body, pool, auth), fields(user_id = %auth.claims.id, user_email = %auth.claims.email))]
pub async fn update(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    auth: AuthorizationService,
) -> Result<HttpResponse, UserUpdateError> {
    let update_user: UpdateUser = body
        .0
        .try_into()
        .map_err(UserUpdateError::ValidationError)?;

    // Get the current user from the database
    let user = get_user_with_id(auth.claims.id, &pool)
        .await
        .context("Failed to retrieve stored user.")?;

    let current_password = update_user.current_password;

    // Check if the supplied current password is correct
    let is_password_correct =
        spawn_blocking_with_tracing(move || verify_password_hash(user.password, current_password))
            .await
            .context("Failed to spawn blocking task.")
            .map_err(UserUpdateError::UnexpectedError)??;

    if is_password_correct == false {
        return Err(UserUpdateError::WrongPasswordError);
    }

    // Update name/email/password, if they are included in the request body
    if let Some(name) = &update_user.name {
        update_user_name(&pool, auth.claims.id, name)
            .await
            .context("Failed to update user name")?;
    }

    if let Some(email) = &update_user.email {
        update_user_email(&pool, auth.claims.id, email)
            .await
            .context("Failed to update user email")?;
    }

    if let Some(password) = &update_user.password {
        update_user_password(&pool, auth.claims.id, password)
            .await
            .context("Failed to update user password")?;
    }

    Ok(HttpResponse::Ok().finish())
}
