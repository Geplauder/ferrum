use actix_http::{encoding::Decoder, Payload};
use ferrum_websocket::messages::WebSocketMessage;

use crate::helpers::{
    get_next_websocket_message, send_websocket_message, TestApplication, TestUser,
};

impl TestApplication {
    pub async fn delete_leave_server(
        &self,
        server_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .delete(&format!("{}/servers/{}/users", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn leave_returns_200_for_valid_request() {
    // Arrange
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Act
    let response = app
        .delete_leave_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn leave_removes_users_servers_entry_from_the_database() {
    // Arrange
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Act
    app.delete_leave_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_user_server = sqlx::query!(
        "SELECT id FROM users_servers WHERE user_id = $1 AND server_id = $2",
        app.test_user().id,
        app.test_server().id
    )
    .fetch_one(&app.db_pool)
    .await;

    assert!(saved_user_server.is_err());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn leave_fails_if_there_is_a_database_error() {
    // Arrange
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    sqlx::query!("ALTER TABLE users_servers DROP COLUMN user_id;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .delete_leave_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn leave_returns_400_when_user_is_not_on_the_server() {
    // Arrange

    // Act
    let response = app
        .delete_leave_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn leave_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Act
    for token in [None, Some("foo".to_string())] {
        let response = app
            .delete_leave_server(app.test_server().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn leave_returns_400_when_user_is_owner_of_the_server() {
    // Arrange

    // Act
    let response = app
        .delete_leave_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn leave_sends_delete_server_to_leaving_user() {
    // Arrange
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

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
    app.delete_leave_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    match message {
        Some(WebSocketMessage::DeleteServer { server_id }) => {
            assert_eq!(app.test_server().id, server_id);
        }
        Some(fallback) => assert!(false, "Received wrong message type: {:#?}", fallback),
        None => assert!(false, "Received no message"),
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn leave_sends_delete_user_to_users_on_server() {
    // Arrange
    let other_user = TestUser::generate();
    other_user.store(&app.db_pool).await;
    let other_user_token = app.jwt.encode(other_user.id, other_user.email);

    app.put_join_server(
        app.test_server().id.to_string(),
        Some(other_user_token.clone()),
    )
    .await;

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
    app.delete_leave_server(app.test_server().id.to_string(), Some(other_user_token))
        .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    match message {
        Some(WebSocketMessage::DeleteUser { user_id, server_id }) => {
            assert_eq!(other_user.id, user_id);
            assert_eq!(app.test_server().id, server_id);
        }
        Some(fallback) => assert!(false, "Received wrong message type: {:#?}", fallback),
        None => assert!(false, "Received no message"),
    }
}