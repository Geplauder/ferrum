use ferrum_db::servers::queries::delete_server;
use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_server_broker_event_sends_delete_server_websocket_message_to_users_on_server() {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    let mut transaction = app.db_pool.begin().await.unwrap();

    delete_server(&mut transaction, app.test_server().id)
        .await
        .unwrap();

    transaction.commit().await.unwrap();

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::DeleteServer {
            server_id: app.test_server().id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::DeleteServer { server_id },
        &mut connection,
        {
            assert_eq!(app.test_server().id, server_id);
        }
    );
}
