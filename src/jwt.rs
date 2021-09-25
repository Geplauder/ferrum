use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{web::Data, FromRequest, HttpResponse, ResponseError};
use futures::future::{err, ok, Ready};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error_chain_fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub email: String,
    iat: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jwt {
    secret: String,
}

impl Jwt {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

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
}

pub fn get_claims(token: &str, secret: &str) -> Option<Claims> {
    let validation = Validation {
        validate_exp: false,
        ..Default::default()
    };

    match jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    ) {
        Ok(value) => Some(value.claims),
        Err(_) => None,
    }
}

pub struct AuthorizationService {
    pub claims: Claims,
}

#[derive(thiserror::Error)]
pub enum AuthorizationError {
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
    type Config = ();
    type Error = AuthorizationError;
    type Future = Ready<Result<AuthorizationService, Self::Error>>;

    fn from_request(
        request: &actix_web::HttpRequest,
        _payload: &mut actix_http::Payload,
    ) -> Self::Future {
        let token = request
            .headers()
            .get("Authorization")
            .and_then(|x| x.to_str().ok())
            .map(|x| x.replace("Bearer ", ""));

        let jwt = request.app_data::<Data<Jwt>>().unwrap();

        if let Some(service) = token.as_ref().and_then(|token| {
            get_claims(token, &jwt.secret).map(|claims| AuthorizationService { claims })
        }) {
            return ok(service);
        }

        let token = request
            .query_string()
            .split('&')
            .map(|parameter| {
                let split: Vec<&str> = parameter.splitn(2, '=').collect();

                (split[0], split[1])
            })
            .filter(|&(key, _)| key == "bearer")
            .collect::<Vec<(&str, &str)>>();

        if token.len() == 1 {
            let (_, value) = token[0];

            if let Some(service) =
                get_claims(value, &jwt.secret).map(|claims| AuthorizationService { claims })
            {
                return ok(service);
            }
        }

        err(AuthorizationError::Unauthorized)
    }
}
