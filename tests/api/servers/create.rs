use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn create_returns_200_for_valid_json_data() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
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

#[actix_rt::test]
async fn create_persists_the_new_server() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
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

#[actix_rt::test]
async fn create_also_joins_owner_to_the_server() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
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

#[actix_rt::test]
async fn create_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
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

#[actix_rt::test]
async fn create_returns_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
    let body = serde_json::json!({});

    // Act
    let response = app
        .post_create_server(body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[actix_rt::test]
async fn create_returns_400_when_data_is_invalid() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
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

#[actix_rt::test]
async fn create_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;
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
