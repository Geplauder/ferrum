use actix_http::{encoding::Decoder, Payload};
use ferrum_shared::broker::BrokerEvent;

use crate::{assert_next_broker_message, helpers::TestApplication};

impl TestApplication {
    pub async fn post_update_channel(
        &self,
        channel_id: String,
        body: &serde_json::Value,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .post(&format!("{}/channels/{}", &self.address, &channel_id));

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
            .post_update_channel(
                app.test_server().default_channel_id.to_string(),
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

    sqlx::query!("ALTER TABLE channels DROP COLUMN name;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_update_channel(
            app.test_server().default_channel_id.to_string(),
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
            .post_update_channel(
                app.test_server().default_channel_id.to_string(),
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
            .post_update_channel(
                app.test_server().default_channel_id.to_string(),
                &body,
                token,
            )
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
        .post_update_channel(
            app.test_server().default_channel_id.to_string(),
            &body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_persists_changes_to_channel() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    // Act
    app.post_update_channel(
        app.test_server().default_channel_id.to_string(),
        &body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_channel = sqlx::query!(
        "SELECT name FROM channels WHERE id = $1",
        app.test_server().default_channel_id,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved channel");

    assert_eq!("FooBar", saved_channel.name);
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_with_empty_body_does_not_change_anything() {
    // Arrange
    let body = serde_json::json!({});

    // Act
    app.post_update_channel(
        app.test_server().id.to_string(),
        &body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_channel = sqlx::query!(
        "SELECT name FROM channels WHERE id = $1",
        app.test_server().default_channel_id,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved channel");

    assert_eq!("general", saved_channel.name);
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn update_sends_update_channel_broker_event() {
    // Arrange
    let body = serde_json::json!({
        "name": "FooBar",
    });

    // Act
    app.post_update_channel(
        app.test_server().default_channel_id.to_string(),
        &body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    assert_next_broker_message!(
        BrokerEvent::UpdateChannel { channel_id },
        &mut app.consumer,
        {
            assert_eq!(app.test_server().default_channel_id, channel_id);
        }
    );
}
