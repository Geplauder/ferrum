use actix_http::{encoding::Decoder, Payload};
use ferrum::jwt::get_claims;

use crate::helpers::TestApplication;

impl TestApplication {
    pub async fn post_login(
        &self,
        body: serde_json::Value,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        self.http_client()
            .post(&format!("{}/login", &self.address))
            .send_json(&body)
            .await
            .expect("Failed to execute request.")
    }
}

#[derive(serde::Deserialize)]
struct LoginResponse {
    token: String,
}

#[geplauder_macros::test(strategy = "User")]
async fn test_login_returns_200_for_valid_json_data() {
    // Arrange
    let body = serde_json::json!({
        "email": app.test_user().email,
        "password": app.test_user().password,
    });

    // Act
    let mut response = app.post_login(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let response_data = response.json::<LoginResponse>().await;
    assert!(response_data.is_ok());

    let response_data = response_data.unwrap();

    let claims = get_claims(&response_data.token, &app.jwt_secret);
    assert!(claims.is_some());

    let claims = claims.unwrap();
    assert_eq!(app.test_user().email, claims.email);
}

#[geplauder_macros::test(strategy = "User")]
async fn login_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "email": app.test_user().email,
        "password": app.test_user().password,
    });

    sqlx::query!("ALTER TABLE users DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_login(body).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[geplauder_macros::test(strategy = "User")]
async fn login_returns_400_when_data_is_missing() {
    // Arrange
    let body = serde_json::json!({
        "email": app.test_user().email,
    });

    // Act
    let response = app.post_login(body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[geplauder_macros::test(strategy = "User")]
async fn login_returns_401_when_data_is_invalid() {
    // Arrange
    let payloads = [
        serde_json::json!({
            "email": app.test_user().email,
            "password": "foobar",
        }),
        serde_json::json!({
            "email": "foo@bar.com",
            "password": app.test_user().password,
        }),
    ];

    for body in payloads {
        // Act
        let response = app.post_login(body).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}
