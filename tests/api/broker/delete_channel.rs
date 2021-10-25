use ferrum_db::channels::queries::delete_channel;
use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_channel_broker_event_sends_delete_channel_websocket_message_to_users_with_access_to_channel(
) {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    let mut transaction = app.db_pool.begin().await.unwrap();

    delete_channel(&mut transaction, app.test_server().default_channel_id)
        .await
        .unwrap();

    transaction.commit().await.unwrap();

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::DeleteChannel {
            channel_id: app.test_server().default_channel_id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::DeleteChannel { channel_id },
        &mut connection,
        {
            assert_eq!(app.test_server().default_channel_id, channel_id);
        }
    );
}
