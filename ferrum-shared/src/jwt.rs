use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{web::Data, FromRequest, HttpResponse, ResponseError};
use futures::future::{err, ok, Ready};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error_chain_fmt;

///
/// Contains information about the user, decoded from the JWT.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub email: String,
    iat: usize,
}

///
/// Helper to encode and decode JWTs.
///
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jwt {
    secret: String,
}

impl Jwt {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    ///
    /// Create a JWT from an users' id and email.
    ///
    pub fn encode(&self, id: Uuid, email: String) -> String {
        let now = SystemTime::now();
        let since_epoch = now.duration_since(UNIX_EPOCH).unwrap();

        jsonwebtoken::encode(
            &Header::default(),
            &Claims {
                id,
                email,
                iat: since_epoch.as_millis() as usize,
            },
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .unwrap()
    }

    ///
    /// Get the claims for a JWT.
    ///
    /// Note: Currently tokens are not checked for expiration.
    ///
    pub fn get_claims(&self, token: &str) -> Option<Claims> {
        // Validate without checking for expiration
        let validation = Validation {
            validate_exp: false,
            ..Default::default()
        };

        // Try to decode the JWT. Either returning the associated claims ore none, if decoding was not possible
        match jsonwebtoken::decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        ) {
            Ok(value) => Some(value.claims),
            Err(_) => None,
        }
    }
}

///
/// This can be used to restrict a route to authenticated users
/// by having it as a parameter in the request handler.
///
/// If the user is unauthenticated, the request will result in a 401 Unauthorized.
///
pub struct AuthorizationService {
    pub claims: Claims,
}

///
/// Possible errors that can occur in the [`AuthorizationService`].
///
/// Due to the nature of that middleware, it only contains an Unauthorized
/// error, which returns a 401 for the request.
///
#[derive(thiserror::Error)]
pub enum AuthorizationError {
    /// User could not be authenticated.
    #[error("Unauthorized")]
    Unauthorized,
}

impl std::fmt::Debug for AuthorizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for AuthorizationError {
    fn error_response(&self) -> actix_web::HttpResponse {
        match self {
            AuthorizationError::Unauthorized => HttpResponse::Unauthorized().finish(),
        }
    }
}

impl FromRequest for AuthorizationService {
    type Error = AuthorizationError;
    type Future = Ready<Result<AuthorizationService, Self::Error>>;

    fn from_request(
        request: &actix_web::HttpRequest,
        _payload: &mut actix_http::Payload,
    ) -> Self::Future {
        // Check if the request contains a Authorization header.
        // If so, it strips the "Bearer " prefix.
        let token = request
            .headers()
            .get("Authorization")
            .and_then(|x| x.to_str().ok())
            .map(|x| x.replace("Bearer ", ""));

        let jwt = request.app_data::<Data<Jwt>>().unwrap();

        // Try to decode the JWT, returning an Unauthorized error if not possible
        if let Some(service) = token.as_ref().and_then(|token| {
            jwt.get_claims(token)
                .map(|claims| AuthorizationService { claims })
        }) {
            return ok(service);
        }

        err(AuthorizationError::Unauthorized)
    }
}
