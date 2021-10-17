use ferrum_db::servers::{
    models::{NewServer, ServerName},
    queries::insert_server,
};
use ferrum_shared::broker::BrokerEvent;
use ferrum_websocket::messages::SerializedWebSocketMessage;

use crate::{assert_next_websocket_message, helpers::publish_broker_message};

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn new_server_broker_event_sends_new_server_websocket_message_to_owner() {
    // Arrange
    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    let mut transaction = app.db_pool.begin().await.unwrap();

    let server = insert_server(
        &mut transaction,
        &NewServer {
            name: ServerName::parse("foobar".to_string()).unwrap(),
        },
        app.test_user().id,
    )
    .await
    .unwrap();

    transaction.commit().await.unwrap();

    // Act
    publish_broker_message(
        &app,
        BrokerEvent::NewServer {
            server_id: server.id,
            user_id: app.test_user().id,
        },
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        SerializedWebSocketMessage::NewServer {
            server: new_server,
            users: _,
            channels: _
        },
        &mut connection,
        {
            assert_eq!(server.id, new_server.id);
        }
    );
}
