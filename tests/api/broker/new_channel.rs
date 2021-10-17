use ferrum_db::channels::{
    models::{ChannelName, NewChannel},
    queries::insert_channel,
};
use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn new_channel_broker_event_sends_new_channel_websocket_message_to_users_on_server() {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    let mut transaction = app.db_pool.begin().await.unwrap();

    let channel = insert_channel(
        &mut transaction,
        &NewChannel {
            name: ChannelName::parse("foobar".to_string()).unwrap(),
        },
        app.test_server().id,
    )
    .await
    .unwrap();

    transaction.commit().await.unwrap();

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::NewChannel {
            channel_id: channel.id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::NewChannel {
            channel: new_channel
        },
        &mut connection,
        {
            assert_eq!("foobar", new_channel.name);
        }
    );
}
