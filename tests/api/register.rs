use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn register_returns_200_for_valid_json_data() {
    // Arrange
    let app = spawn_app(BootstrapType::Default).await;
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foo@bar.com",
        "password": "foobar123"
    });

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn register_persists_the_new_user() {
    // Arrange
    let app = spawn_app(BootstrapType::Default).await;
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foo@bar.com",
        "password": "foobar123"
    });

    // Act
    app.post_register(body).await;

    // Assert
    let saved_user = sqlx::query!("SELECT username, email FROM users",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user");

    assert_eq!("foobar", saved_user.username);
    assert_eq!("foo@bar.com", saved_user.email);
}

#[actix_rt::test]
async fn register_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::Default).await;
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foo@bar.com",
        "password": "foobar123"
    });

    sqlx::query!("ALTER TABLE users DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[actix_rt::test]
async fn register_returns_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app(BootstrapType::Default).await;
    let body = serde_json::json!({
        "name": "foobar",
        "password": "foobar123"
    });

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[actix_rt::test]
async fn register_returns_400_when_data_is_invalid() {
    // Arrange
    let app = spawn_app(BootstrapType::Default).await;
    let body = serde_json::json!({
        "name": "foobar",
        "email": "foobar.com",
        "password": "foobar123"
    });

    // Act
    let response = app.post_register(body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}
