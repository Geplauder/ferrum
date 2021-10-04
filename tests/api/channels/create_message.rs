use actix_http::{encoding::Decoder, Payload};
use ferrum_websocket::messages::WebSocketMessage;
use uuid::Uuid;

use crate::helpers::{get_next_websocket_message, send_websocket_message, TestApplication};

impl TestApplication {
    pub async fn post_create_channel_message(
        &self,
        channel_id: String,
        body: serde_json::Value,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self.http_client().post(&format!(
            "{}/channels/{}/messages",
            &self.address, channel_id
        ));

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
async fn create_message_returns_200_for_valid_request_data() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_persists_the_new_message() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    sqlx::query!("DELETE FROM messages")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    app.post_create_channel_message(
        app.test_server().default_channel_id.to_string(),
        body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_message = sqlx::query!("SELECT content, channel_id FROM messages")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved channel message");

    assert_eq!("foobar", saved_message.content);
    assert_eq!(
        app.test_server().default_channel_id,
        saved_message.channel_id
    );
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    sqlx::query!("ALTER TABLE messages DROP COLUMN content;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_returns_404_when_channel_id_is_invalid() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message("foo".to_string(), body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_returns_403_when_channel_id_is_not_found() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message(
            Uuid::new_v4().to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_returns_400_when_data_is_missing() {
    // Arrange
    let body = serde_json::json!({});

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_returns_400_when_data_is_invalid() {
    // Arrange

    for data in ["", &(0..=1001).map(|_| "x").collect::<String>()] {
        let body = serde_json::json!({ "content": data });

        // Act
        let response = app
            .post_create_channel_message(
                app.test_server().default_channel_id.to_string(),
                body,
                Some(app.test_user_token()),
            )
            .await;

        // Assert
        assert_eq!(400, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .post_create_channel_message(
                app.test_server().default_channel_id.to_string(),
                body.clone(),
                token,
            )
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn create_message_returns_403_when_user_has_no_access_to_the_channel() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_sends_websocket_message_to_ready_users() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
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
    app.post_create_channel_message(
        app.test_server().default_channel_id.to_string(),
        body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    match message {
        Some(WebSocketMessage::NewMessage { message }) => {
            assert_eq!("foobar", message.content);
            assert_eq!(app.test_user().id, message.user.id);
        }
        Some(fallback) => assert!(false, "Received wrong message type: {:#?}", fallback),
        None => assert!(false, "Received no message"),
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_message_does_not_send_websocket_message_to_non_bootstrapped_users() {
    // Arrange
    let body = serde_json::json!({
        "content": "foobar"
    });

    let (_response, mut connection) = app.websocket().await;

    // Act
    app.post_create_channel_message(
        app.test_server().default_channel_id.to_string(),
        body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    assert!(
        message.is_none(),
        "Received a websocket message: {:#?}",
        message
    );
}
