use actix_http::{encoding::Decoder, Payload};

use crate::helpers::TestApplication;

impl TestApplication {
    pub async fn post_register(
        &self,
        body: serde_json::Value,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        self.http_client()
            .post(&format!("{}/register", &self.address))
            .send_json(&body)
            .await
            .expect("Failed to execute request.")
    }
}

#[geplauder_macros::test]
async fn register_returns_200_for_valid_json_data() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foo@bar.com",
        "password": "foobar123"
    });

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[geplauder_macros::test]
async fn register_persists_the_new_user() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foo@bar.com",
        "password": "foobar123"
    });

    // Act
    app.post_register(body).await;

    // Assert
    let saved_user = sqlx::query!("SELECT username, email FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user");

    assert_eq!("foobar", saved_user.username);
    assert_eq!("foo@bar.com", saved_user.email);
}

#[geplauder_macros::test]
async fn register_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foo@bar.com",
        "password": "foobar123"
    });

    sqlx::query!("ALTER TABLE users DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[geplauder_macros::test]
async fn register_returns_400_when_data_is_missing() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
        "password": "foobar123"
    });

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[geplauder_macros::test]
async fn register_returns_400_when_data_is_invalid() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foobar.com",
        "password": "foobar123"
    });

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}
