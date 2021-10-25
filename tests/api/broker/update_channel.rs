use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_channel_broker_event_sends_update_channel_websocket_message_to_users_with_access_to_channel(
) {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::UpdateChannel {
            channel_id: app.test_server().default_channel_id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::UpdateChannel { channel },
        &mut connection,
        {
            assert_eq!(app.test_server().default_channel_id, channel.id);
        }
    );
}
