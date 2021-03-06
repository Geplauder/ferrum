use actix_http::{encoding::Decoder, Payload};
use ferrum_shared::broker::BrokerEvent;
use uuid::Uuid;

use crate::{assert_next_broker_message, helpers::TestApplication};

impl TestApplication {
    pub async fn delete_channel(
        &self,
        channel_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .delete(&format!("{}/channels/{}", &self.address, channel_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_returns_200_for_valid_request() {
    // Arrange

    // Act
    let response = app
        .delete_channel(
            app.test_server().default_channel_id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_deletes_the_server_from_the_database() {
    // Arrange

    // Act
    app.delete_channel(
        app.test_server().default_channel_id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_channels = sqlx::query!("SELECT name, server_id FROM channels")
        .fetch_all(&app.db_pool)
        .await
        .expect("Failed to fetch channel count");

    assert!(saved_channels.is_empty());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .delete_channel(app.test_server().default_channel_id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn delete_channel_returns_403_for_users_without_access() {
    // Arrange

    // Act
    let response = app
        .delete_channel(
            app.test_server().default_channel_id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_returns_404_when_channel_id_is_invalid() {
    // Arrange

    // Act
    let response = app
        .delete_channel("foobar".to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_returns_500_when_channel_id_is_not_found() {
    // Arrange

    // Act
    let response = app
        .delete_channel(Uuid::new_v4().to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_sends_delete_channel_broker_event() {
    // Arrange

    // Act
    app.delete_channel(
        app.test_server().default_channel_id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    assert_next_broker_message!(
        BrokerEvent::DeleteChannel {
            server_id,
            channel_id
        },
        &mut app.consumer,
        {
            assert_eq!(app.test_server().id, server_id);
            assert_eq!(app.test_server().default_channel_id, channel_id);
        }
    );
}
