use ferrum_db::messages::{
    models::{MessageContent, NewMessage},
    queries::insert_message,
};
use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn new_message_broker_event_sends_new_message_websocket_message_to_users_in_channel() {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    let mut transaction = app.db_pool.begin().await.unwrap();

    let message = insert_message(
        &mut transaction,
        &app.db_pool,
        &NewMessage {
            content: MessageContent::parse("foobar".to_string()).unwrap(),
        },
        app.test_server().default_channel_id,
        app.test_user().id,
    )
    .await
    .unwrap();

    transaction.commit().await.unwrap();

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::NewMessage {
            channel_id: app.test_server().default_channel_id,
            message_id: message.id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::NewMessage { message },
        &mut connection,
        {
            assert_eq!("foobar", message.content);
            assert_eq!(app.test_server().default_channel_id, message.channel_id);
            assert_eq!(app.test_user().id, message.user.id);
        }
    );
}
