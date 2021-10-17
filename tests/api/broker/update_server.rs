use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_server_broker_event_sends_update_server_websocket_message_to_users_on_server() {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::UpdateServer {
            server_id: app.test_server().id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::UpdateServer { server },
        &mut connection,
        {
            assert_eq!(app.test_server().id, server.id);
        }
    );
}
