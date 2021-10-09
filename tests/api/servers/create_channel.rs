use actix_http::{encoding::Decoder, Payload};
use ferrum_websocket::messages::WebSocketMessage;
use uuid::Uuid;

use crate::{
    assert_next_websocket_message,
    helpers::{get_next_websocket_message, send_websocket_message, TestApplication},
};

impl TestApplication {
    pub async fn post_create_server_channel(
        &self,
        server_id: String,
        body: serde_json::Value,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .post(&format!("{}/servers/{}/channels", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client
            .send_json(&body)
            .await
            .expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_returns_200_for_valid_request_data() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    // Act
    let response = app
        .post_create_server_channel(
            app.test_server().id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_persists_the_new_channel() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    sqlx::query!("DELETE FROM channels")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    app.post_create_server_channel(
        app.test_server().id.to_string(),
        body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_channel = sqlx::query!("SELECT name, server_id FROM channels")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved server channel");

    assert_eq!("foobar", saved_channel.name);
    assert_eq!(app.test_server().id, saved_channel.server_id);
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    sqlx::query!("ALTER TABLE channels DROP COLUMN name;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_create_server_channel(
            app.test_server().id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_returns_404_when_server_id_is_invalid() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    // Act
    let response = app
        .post_create_server_channel("foo".to_string(), body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_returns_500_when_server_id_is_not_found() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    // Act
    let response = app
        .post_create_server_channel(
            Uuid::new_v4().to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_returns_400_when_data_is_missing() {
    // Arrange
    let body = serde_json::json!({});

    // Act
    let response = app
        .post_create_server_channel(
            app.test_server().id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_returns_400_when_data_is_invalid() {
    // Arrange

    for data in ["", "foo", &(0..=33).map(|_| "x").collect::<String>()] {
        let body = serde_json::json!({ "name": data });

        // Act
        let response = app
            .post_create_server_channel(
                app.test_server().id.to_string(),
                body,
                Some(app.test_user_token()),
            )
            .await;

        // Assert
        assert_eq!(400, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let body = serde_json::json !({
        "name": "foobar",
    });

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .post_create_server_channel(app.test_server().id.to_string(), body.to_owned(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn create_channel_returns_403_when_user_is_not_owner_of_the_server() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    // Act
    let response = app
        .post_create_server_channel(
            app.test_server().id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_channel_sends_new_channel_to_authenticated_websocket_users() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    let (_response, mut connection) = app.websocket().await;

    send_websocket_message(
        &mut connection,
        WebSocketMessage::Identify {
            bearer: app.test_user_token(),
        },
    )
    .await;

    get_next_websocket_message(&mut connection).await; // Accept the "Ready" message

    // Act
    app.post_create_server_channel(
        app.test_server().id.to_string(),
        body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        WebSocketMessage::NewChannel {
            channel: new_channel
        },
        &mut connection,
        {
            assert_eq!("foobar", new_channel.name);
        }
    );
}

// TODO: Add test to ensure that users not on the server do not get messaged about it
