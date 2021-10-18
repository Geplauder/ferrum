use actix_http::ws;
use claim::assert_ok;
use ferrum_websocket::messages::SerializedWebSocketMessage;
use futures::SinkExt;

use crate::{
    assert_next_websocket_message, assert_no_next_websocket_message,
    helpers::{get_next_websocket_message, send_websocket_message},
};

#[ferrum_macros::test(strategy = "User")]
async fn websocket_is_valid_for_valid_request() {
    // Arrange

    // Act
    let (response, mut _connection) = app.websocket().await;

    // Assert
    assert_eq!(101, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn websocket_closes_successfully() {
    // Arrange
    let (_response, mut connection) = app.websocket().await;

    send_websocket_message(
        &mut connection,
        SerializedWebSocketMessage::Identify {
            bearer: app.test_user_token(),
        },
    )
    .await;

    get_next_websocket_message(&mut connection).await;

    // Act
    let close_request = connection.send(ws::Message::Close(None)).await;

    // Assert
    assert_ok!(close_request);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await; // Wait until server processed that message
}

#[ferrum_macros::test(strategy = "User")]
async fn websocket_receives_ready_message_after_successfull_identify() {
    // Arrange
    let (_response, mut connection) = app.websocket().await;

    // Act
    send_websocket_message(
        &mut connection,
        SerializedWebSocketMessage::Identify {
            bearer: app.test_user_token(),
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(SerializedWebSocketMessage::Ready, &mut connection, ());
}

#[ferrum_macros::test(strategy = "User")]
async fn websocket_does_not_receive_ready_message_after_missing_or_invalid_bearer_token_in_identify_message(
) {
    // Arrange
    let (_response, mut connection) = app.websocket().await;

    for token in ["".to_string(), "foo".to_string()] {
        // Act
        send_websocket_message(
            &mut connection,
            SerializedWebSocketMessage::Identify { bearer: token },
        )
        .await;

        // Assert
        assert_no_next_websocket_message!(&mut connection);
    }
}

#[ferrum_macros::test(strategy = "User")]
async fn websocket_responds_to_ping_with_pong() {
    // Arrange
    let (_response, mut connection) = app.websocket().await;

    // Act
    send_websocket_message(&mut connection, SerializedWebSocketMessage::Ping).await;

    // Assert
    assert_next_websocket_message!(SerializedWebSocketMessage::Pong, &mut connection, ());
}

#[ferrum_macros::test(strategy = "User")]
async fn websocket_survives_malformed_messages() {
    // Arrange
    let (_response, mut connection) = app.websocket().await;

    // Act
    connection
        .send(ws::Message::Text("foobar".into()))
        .await
        .unwrap();

    // Assert
    let response = app
        .http_client()
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
}
