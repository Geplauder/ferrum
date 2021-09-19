use uuid::Uuid;

use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn create_message_returns_200_for_valid_request_data() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn create_message_persists_the_new_message() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    sqlx::query!("DELETE FROM messages")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    app.post_create_channel_message(
        app.test_server().default_channel_id.to_string(),
        body,
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_message = sqlx::query!("SELECT content, channel_id FROM messages")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved channel message");

    assert_eq!("foobar", saved_message.content);
    assert_eq!(
        app.test_server().default_channel_id,
        saved_message.channel_id
    );
}

#[actix_rt::test]
async fn create_message_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    sqlx::query!("ALTER TABLE messages DROP COLUMN content;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[actix_rt::test]
async fn create_message_returns_404_when_channel_id_is_invalid() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message("foo".to_string(), body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[actix_rt::test]
async fn create_message_returns_500_when_channel_id_is_not_found() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message(
            Uuid::new_v4().to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[actix_rt::test]
async fn create_message_returns_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({});

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[actix_rt::test]
async fn create_message_returns_400_when_data_is_invalid() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    for data in ["", &(0..=1001).map(|_| "x").collect::<String>()] {
        let body = serde_json::json!({ "content": data });

        // Act
        let response = app
            .post_create_channel_message(
                app.test_server().default_channel_id.to_string(),
                body,
                Some(app.test_user_token()),
            )
            .await;

        // Assert
        assert_eq!(400, response.status().as_u16());
    }
}

#[actix_rt::test]
async fn create_message_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .post_create_channel_message(
                app.test_server().default_channel_id.to_string(),
                body.clone(),
                token,
            )
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[actix_rt::test]
#[ignore] // TODO: Remove when feature is implemented
async fn create_message_returns_401_when_user_has_no_access_to_the_channel() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;
    let body = serde_json::json!({
        "content": "foobar"
    });

    // Act
    let response = app
        .post_create_channel_message(
            app.test_server().default_channel_id.to_string(),
            body,
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(401, response.status().as_u16());
}
