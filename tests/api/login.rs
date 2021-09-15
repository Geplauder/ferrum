use crate::helpers::spawn_app;

#[actix_rt::test]
async fn login_returns_200_for_valid_json_data() {
    // Arrange
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": app.test_user.email,
        "password": app.test_user.password,
    });

    // Act
    let response = app.post_login(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn login_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": app.test_user.email,
        "password": app.test_user.password,
    });

    sqlx::query!("ALTER TABLE users DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_login(body).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[actix_rt::test]
async fn login_returns_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;

    let body = serde_json::json!({
        "email": app.test_user.email,
    });

    // Act
    let response = app.post_login(body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[actix_rt::test]
async fn login_returns_401_when_data_is_invalid() {
    // Arrange
    let app = spawn_app().await;

    let payloads = [
        serde_json::json!({
            "email": app.test_user.email,
            "password": "foobar",
        }),
        serde_json::json!({
            "email": "foo@bar.com",
            "password": app.test_user.password,
        }),
    ];

    for body in payloads {
        // Act
        let response = app.post_login(body).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}
