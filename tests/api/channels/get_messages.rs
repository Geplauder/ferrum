use actix_http::{encoding::Decoder, Payload};
use ferrum_shared::messages::MessageResponse;
use sqlx::PgPool;
use uuid::Uuid;

use crate::helpers::{TestApplication, TestServer, TestUser};

impl TestApplication {
    pub async fn get_channel_messages(
        &self,
        channel_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self.http_client().get(&format!(
            "{}/channels/{}/messages",
            &self.address, channel_id
        ));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_messages_returns_200_for_valid_request() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    // Act
    let response = app
        .get_channel_messages(
            app.test_server().default_channel_id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_messages_returns_valid_data_for_valid_request() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    // Act
    let response = app
        .get_channel_messages(
            app.test_server().default_channel_id.to_string(),
            Some(app.test_user_token()),
        )
        .await
        .json::<Vec<MessageResponse>>()
        .await
        .unwrap();

    // Assert
    assert_eq!(1, response.len());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_messages_fails_if_there_is_a_database_error() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    sqlx::query!("ALTER TABLE messages DROP COLUMN content;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .get_channel_messages(
            app.test_server().default_channel_id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_messages_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .get_channel_messages(app.test_server().default_channel_id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn get_messages_returns_403_for_users_without_access() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    // Act
    let response = app
        .get_channel_messages(
            app.test_server().default_channel_id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_messages_returns_404_when_channel_id_is_invalid() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    // Act
    let response = app
        .get_channel_messages("foobar".to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_messages_returns_403_when_channel_id_is_not_found() {
    // Arrange
    add_message_to_channel(
        &app.test_server(),
        app.test_server().default_channel_id,
        &app.db_pool,
    )
    .await;

    // Act
    let response = app
        .get_channel_messages(Uuid::new_v4().to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

async fn add_message_to_channel(server: &TestServer, channel_id: Uuid, pool: &PgPool) {
    let user = TestUser::generate();
    user.store(pool).await;

    server.add_user(user.id, pool).await;

    sqlx::query!(
        r#"
        INSERT INTO messages (id, channel_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        channel_id,
        user.id,
        Uuid::new_v4().to_string(),
    )
    .execute(pool)
    .await
    .unwrap();
}
