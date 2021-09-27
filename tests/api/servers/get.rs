use actix_http::{encoding::Decoder, Payload};
use ferrum::domain::servers::Server;
use uuid::Uuid;

use crate::helpers::{spawn_app, BootstrapType, TestApplication};

impl TestApplication {
    pub async fn get_server(
        &self,
        server_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .get(&format!("{}/servers/{}", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[actix_rt::test]
async fn get_server_returns_200_for_valid_request() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Act
    let response = app
        .get_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn get_server_returns_valid_data_for_valid_response() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Act
    let response = app
        .get_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await
        .json::<Server>()
        .await
        .unwrap();

    // Assert
    assert_eq!(app.test_server().id, response.id);
    assert_eq!(app.test_server().name, response.name);
}

#[actix_rt::test]
async fn get_server_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    sqlx::query!("ALTER TABLE servers DROP COLUMN name;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .get_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[actix_rt::test]
async fn get_server_returns_404_when_server_id_is_invalid() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Act
    let response = app
        .get_server("foobar".to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[actix_rt::test]
async fn get_server_returns_401_when_server_id_is_not_found() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Act
    let response = app
        .get_server(Uuid::new_v4().to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(401, response.status().as_u16());
}

#[actix_rt::test]
async fn get_server_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .get_server(app.test_server().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[actix_rt::test]
async fn get_server_returns_401_for_users_without_access() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;

    // Act
    let response = app
        .get_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(401, response.status().as_u16());
}
