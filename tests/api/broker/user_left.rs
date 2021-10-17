use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{
    assert_next_websocket_message,
    helpers::{publish_broker_message, TestUser},
};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_user_broker_event_sends_delete_user_websocket_message_to_users_on_server() {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    let other_user = TestUser::generate();
    other_user.store(&app.db_pool).await;

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::UserLeft {
            server_id: app.test_server().id,
            user_id: other_user.id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::DeleteUser { server_id, user_id },
        &mut connection,
        {
            assert_eq!(app.test_server().id, server_id);
            assert_eq!(other_user.id, user_id);
        }
    );
}
