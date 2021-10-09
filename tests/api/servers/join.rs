use actix_http::{encoding::Decoder, Payload};
use ferrum_websocket::messages::WebSocketMessage;

use crate::helpers::{
    get_next_websocket_message, send_websocket_message, TestApplication, TestUser,
};

impl TestApplication {
    pub async fn put_join_server(
        &self,
        server_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .put(&format!("{}/servers/{}", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_returns_200_for_valid_request() {
    // Arrange

    // Act
    let response = app
        .put_join_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_persists_the_new_user_server_entry() {
    // Arrange

    // Act
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_user_server = sqlx::query!(
        "SELECT user_id, server_id FROM users_servers WHERE user_id = $1 AND server_id = $2",
        app.test_user().id,
        app.test_server().id,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved users_servers entry");

    assert_eq!(app.test_user().id, saved_user_server.user_id);
    assert_eq!(app.test_server().id, saved_user_server.server_id);
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_fails_if_there_is_a_database_error() {
    // Arrange
    sqlx::query!("ALTER TABLE users_servers DROP COLUMN user_id;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .put_join_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_returns_203_when_user_is_already_joined() {
    // Arrange
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Act
    let response = app
        .put_join_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(204, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .put_join_server(app.test_server().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_sends_new_server_to_joining_user() {
    // Arrange
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
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    match message {
        Some(WebSocketMessage::NewServer {
            server: new_server,
            channels,
            users,
        }) => {
            assert_eq!(app.test_server().name, new_server.name);
            assert_eq!(1, channels.len());
            assert_eq!(2, users.len());
        }
        Some(fallback) => assert!(false, "Received wrong message type: {:#?}", fallback),
        None => assert!(false, "Received no message"),
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn join_does_not_send_new_user_websocket_message_to_new_user() {
    // Arrange
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
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    get_next_websocket_message(&mut connection).await; // New Server websocket message

    let message = get_next_websocket_message(&mut connection).await; // No further websocket messages should be received

    assert!(
        message.is_none(),
        "Received a websocket message: {:#?}",
        message
    );
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn join_sends_new_user_to_existing_users() {
    // Arrange
    let new_user = TestUser::generate();
    new_user.store(&app.db_pool).await;
    let new_user_token = app.jwt.encode(new_user.id, new_user.email);

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
    app.put_join_server(app.test_server().id.to_string(), Some(new_user_token))
        .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    match message {
        Some(WebSocketMessage::NewUser { server_id: _, user }) => {
            assert_eq!(new_user.id, user.id);
        }
        Some(fallback) => assert!(false, "Received wrong message type: {:#?}", fallback),
        None => assert!(false, "Received no message"),
    }
}

// TODO: Add test for invalid server_id
