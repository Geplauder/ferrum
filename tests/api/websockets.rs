use actix_http::ws;
use futures::SinkExt;

use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn websocket_is_valid_for_valid_request() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;

    // Act
    let (response, mut _connection) = app
        .websocket(Some(app.test_user_token()))
        .await
        .connect()
        .await
        .unwrap();

    // Assert
    assert_eq!(101, response.status().as_u16());
}

#[actix_rt::test]
async fn websocket_closes_successfully() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;

    let (_response, mut connection) = app
        .websocket(Some(app.test_user_token()))
        .await
        .connect()
        .await
        .unwrap();

    // Act
    let close_request = connection.send(ws::Message::Close(None)).await;

    // Assert
    assert!(close_request.is_ok());
}

#[actix_rt::test]
async fn websocket_fails_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;

    // Act
    for token in [None, Some("foo".to_string())] {
        // Act
        let ws = app.websocket(token).await.connect().await;

        // Assert
        assert!(ws.is_err());
    }
}
