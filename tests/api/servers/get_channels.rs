use actix_http::{encoding::Decoder, Payload};
use ferrum_shared::channels::ChannelResponse;
use sqlx::PgPool;
use uuid::Uuid;

use crate::helpers::{TestApplication, TestServer};

impl TestApplication {
    pub async fn get_server_channels(
        &self,
        server_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .get(&format!("{}/servers/{}/channels", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_channels_returns_200_for_valid_request() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    // Act
    let response = app
        .get_server_channels(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_channels_returns_valid_data_for_valid_request() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    // Act
    let response = app
        .get_server_channels(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await
        .json::<Vec<ChannelResponse>>()
        .await
        .unwrap();

    // Assert
    assert_eq!(2, response.len());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_channels_fails_if_there_is_a_database_error() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    sqlx::query!("ALTER TABLE channels DROP COLUMN name;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .get_server_channels(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_channels_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .get_server_channels(app.test_server().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn get_channels_returns_403_when_user_does_not_have_access_to_server() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    // Act
    let response = app
        .get_server_channels(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_channels_returns_404_when_server_id_is_invalid() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    // Act
    let response = app
        .get_server_channels("foo".to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_channels_returns_403_when_server_id_is_not_found() {
    // Arrange
    add_channel_to_server(&app.test_server(), &app.db_pool).await;

    // Act
    let response = app
        .get_server_channels(Uuid::new_v4().to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

async fn add_channel_to_server(server: &TestServer, pool: &PgPool) {
    server
        .add_channel(Uuid::new_v4(), &Uuid::new_v4().to_string(), pool)
        .await;
}
