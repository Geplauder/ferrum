use actix_http::{encoding::Decoder, Payload};
use ferrum_websocket::messages::WebSocketMessage;

use crate::helpers::{get_next_websocket_message, send_websocket_message, TestApplication};

impl TestApplication {
    pub async fn post_create_server(
        &self,
        body: serde_json::Value,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .post(&format!("{}/servers", &self.address));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client
            .send_json(&body)
            .await
            .expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "User")]
async fn create_returns_200_for_valid_json_data() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
    });

    // Act
    let response = app
        .post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn create_persists_the_new_server() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
    });

    // Act
    app.post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    let saved_server = sqlx::query!("SELECT name, owner_id FROM servers",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved server");

    assert_eq!("foobar", saved_server.name);
    assert_eq!(app.test_user().id, saved_server.owner_id);
}

#[ferrum_macros::test(strategy = "User")]
async fn create_also_joins_owner_to_the_server() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
    });

    // Act
    app.post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    let saved_users_servers = sqlx::query!("SELECT user_id, server_id FROM users_servers")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved users_servers entry.");

    assert_eq!(app.test_user().id, saved_users_servers.user_id);
}

#[ferrum_macros::test(strategy = "User")]
async fn create_also_creates_default_server_channel() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    // Act
    app.post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    let saved_channel = sqlx::query!("SELECT server_id, name FROM channels")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved default server channel.");

    assert_eq!("general", saved_channel.name);
}

#[ferrum_macros::test(strategy = "User")]
async fn create_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar",
    });

    sqlx::query!("ALTER TABLE servers DROP COLUMN name;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn create_returns_400_when_data_is_missing() {
    // Arrange
    let body = serde_json::json!({});

    // Act
    let response = app
        .post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn create_returns_400_when_data_is_invalid() {
    // Arrange
    let body = serde_json::json !({
        "name": "foo",
    });

    // Act
    let response = app
        .post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn create_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let body = serde_json::json !({
        "name": "foobar",
    });

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app.post_create_server(body.to_owned(), token).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn create_sends_new_server_to_owner_per_websocket() {
    // Arrange
    let body = serde_json::json!({
        "name": "foobar"
    });

    let (_response, mut connection) = app.websocket().await;

    send_websocket_message(
        &mut connection,
        WebSocketMessage::Identify {
            bearer: app.test_user_token(),
        },
    )
    .await;

    get_next_websocket_message(&mut connection).await; // Accept the "Ready" message

    // Act
    app.post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    let message = get_next_websocket_message(&mut connection).await;

    match message {
        Some(WebSocketMessage::NewServer {
            server: new_server,
            channels,
            users,
        }) => {
            assert_eq!("foobar", new_server.name);
            assert_eq!(1, channels.len());
            assert_eq!(1, users.len());
        }
        Some(fallback) => assert!(false, "Received wrong message type: {:#?}", fallback),
        None => assert!(false, "Received no message"),
    }
}
