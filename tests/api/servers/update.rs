use actix_http::{encoding::Decoder, Payload};
use ferrum_websocket::messages::WebSocketMessage;

use crate::{
    assert_next_websocket_message, assert_no_next_websocket_message,
    helpers::{TestApplication, TestUser},
};

impl TestApplication {
    pub async fn post_update_server(
        &self,
        server_id: String,
        body: &serde_json::Value,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .post(&format!("{}/servers/{}", &self.address, &server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client
            .send_json(body)
            .await
            .expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_returns_200_for_valid_json_data() {
    // Arrange
    let bodies = [serde_json::json!({
        "name": "FooBar",
    })];

    // Act
    for body in bodies {
        let response = app
            .post_update_server(
                app.test_server().id.to_string(),
                &body,
                Some(app.test_user_token()),
            )
            .await;

        assert_eq!(200, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    sqlx::query!("ALTER TABLE servers DROP COLUMN name;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_update_server(
            app.test_server().id.to_string(),
            &body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_returns_400_for_invalid_json_data() {
    // Arrange
    let bodies = [serde_json::json!({
        "name": "fo",
    })];

    // Act
    for body in bodies {
        let response = app
            .post_update_server(
                app.test_server().id.to_string(),
                &body,
                Some(app.test_user_token()),
            )
            .await;

        assert_eq!(400, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .post_update_server(app.test_server().id.to_string(), &body, token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn update_returns_403_when_user_has_is_not_the_owner() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    // Act
    let response = app
        .post_update_server(
            app.test_server().id.to_string(),
            &body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_persists_changes_to_server() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    // Act
    app.post_update_server(
        app.test_server().id.to_string(),
        &body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_server = sqlx::query!(
        "SELECT name FROM servers WHERE id = $1",
        app.test_server().id,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved server");

    assert_eq!("FooBar", saved_server.name);
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_sends_update_server_websockt_message_to_users_on_server() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    let (_, mut connection) = app
        .get_ready_websocket_connection(app.test_user_token())
        .await;

    // Act
    app.post_update_server(
        app.test_server().id.to_string(),
        &body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    assert_next_websocket_message!(
        WebSocketMessage::UpdateServer { server },
        &mut connection,
        {
            assert_eq!(app.test_server().id, server.id);
            assert_eq!("FooBar", server.name);
        }
    );
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_does_not_send_update_websocket_message_to_users_not_on_server() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    let other_user = TestUser::generate();
    other_user.store(&app.db_pool).await;

    let (_response, mut connection) = app
        .get_ready_websocket_connection(app.jwt.encode(other_user.id, other_user.email))
        .await;

    // Act
    app.post_update_server(
        app.test_server().id.to_string(),
        &body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    assert_no_next_websocket_message!(&mut connection);
}
