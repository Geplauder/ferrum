use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn join_returns_200_for_valid_request() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;

    // Act
    let response = app
        .put_join_server(
            app.test_server.as_ref().unwrap().id.to_string(),
            Some(app.test_user_token.as_ref().unwrap().to_owned()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn join_persists_the_new_user_server_entry() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;

    // Act
    app.put_join_server(
        app.test_server.as_ref().unwrap().id.to_string(),
        Some(app.test_user_token.as_ref().unwrap().to_owned()),
    )
    .await;

    // Assert
    let saved_user_server = sqlx::query!(
        "SELECT user_id, server_id FROM users_servers WHERE user_id = $1 AND server_id = $2",
        app.test_user.as_ref().unwrap().id,
        app.test_server.as_ref().unwrap().id,
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved users_servers entry");

    assert_eq!(
        app.test_user.as_ref().unwrap().id,
        saved_user_server.user_id
    );
    assert_eq!(
        app.test_server.as_ref().unwrap().id,
        saved_user_server.server_id
    );
}

#[actix_rt::test]
async fn join_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;

    sqlx::query!("ALTER TABLE users_servers DROP COLUMN user_id;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .put_join_server(
            app.test_server.as_ref().unwrap().id.to_string(),
            Some(app.test_user_token.as_ref().unwrap().to_owned()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[actix_rt::test]
async fn join_returns_203_when_user_is_already_joined() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;
    app.put_join_server(
        app.test_server.as_ref().unwrap().id.to_string(),
        Some(app.test_user_token.as_ref().unwrap().to_owned()),
    )
    .await;

    // Act
    let response = app
        .put_join_server(
            app.test_server.as_ref().unwrap().id.to_string(),
            Some(app.test_user_token.as_ref().unwrap().to_owned()),
        )
        .await;

    // Assert
    assert_eq!(204, response.status().as_u16());
}

#[actix_rt::test]
async fn join_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOtherServer).await;

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .put_join_server(app.test_server.as_ref().unwrap().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

// TODO: Add test for invalid server_id
