use ferrum::domain::users::User;

use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn current_user_returns_200_for_valid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;

    // Act
    let response = app
        .get_users(Some(app.test_user_token.as_ref().unwrap().to_owned()))
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let user_data = response.json::<User>().await.unwrap();

    assert_eq!(app.test_user.as_ref().unwrap().id, user_data.id);
    assert_eq!(app.test_user.as_ref().unwrap().email, user_data.email);
    assert_eq!(app.test_user.as_ref().unwrap().name, user_data.username);
}

#[actix_rt::test]
async fn current_user_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app.get_users(token).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[actix_rt::test]
async fn current_user_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::User).await;

    sqlx::query!("ALTER TABLE users DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .get_users(Some(app.test_user_token.as_ref().unwrap().to_owned()))
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}
