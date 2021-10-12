use actix_http::{encoding::Decoder, Payload};
use ferrum_db::users::models::verify_password_hash;

use crate::helpers::TestApplication;

impl TestApplication {
    pub async fn post_update_user(
        &self,
        body: &serde_json::Value,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self.http_client().post(&format!("{}/users", &self.address));

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
async fn update_returns_200_for_valid_json_data() {
    // Arrange
    let bodies = [
        serde_json::json!({
            "name": "FooBar",
            "current_password": app.test_user().password
        }),
        serde_json::json!({
            "email": "foo@bar.com",
            "current_password": app.test_user().password
        }),
        serde_json::json!({
            "password": "foobar123",
            "current_password": app.test_user().password
        }),
    ];

    // Act
    for body in bodies {
        let response = app
            .post_update_user(&body, Some(app.test_user_token()))
            .await;

        // Assert
        assert_eq!(200, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "User")]
async fn update_fails_if_there_is_a_database_error() {
    // Arrange
    let body = serde_json::json!({
        "email": "foo@bar.com",
        "current_password": app.test_user().password
    });

    sqlx::query!("ALTER TABLE users DROP COLUMN email;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .post_update_user(&body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn update_returns_400_when_required_data_is_missing() {
    // Arrange
    let body = serde_json::json!({
        "email": "foo@bar.com",
    });

    // Act
    let response = app
        .post_update_user(&body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn update_returns_400_for_invalid_json_data() {
    // Arrange
    let bodies = [
        serde_json::json!({
            "name": "f",
            "current_password": app.test_user().password
        }),
        serde_json::json!({
            "email": "foobar",
            "current_password": app.test_user().password
        }),
        serde_json::json!({
            "password": "foo",
            "current_password": app.test_user().password
        }),
    ];

    // Act
    for body in bodies {
        let response = app
            .post_update_user(&body, Some(app.test_user_token()))
            .await;

        // Assert
        assert_eq!(400, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "User")]
async fn update_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let body = serde_json::json!({
        "email": "foo@bar.com",
        "current_password": app.test_user().password
    });

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app.post_update_user(&body, token).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "User")]
async fn update_returns_403_when_current_password_is_wrong() {
    // Arrange
    let body = serde_json::json!({
        "email": "foo@bar.com",
        "current_password": "foo"
    });

    // Act
    let response = app
        .post_update_user(&body, Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "User")]
async fn update_persists_changes_to_user() {
    // Arrange
    let body = serde_json::json!({
        "email": "foo@bar.com",
        "name": "FooBar",
        "password": "foobar123",
        "current_password": app.test_user().password
    });

    // Act
    app.post_update_user(&body, Some(app.test_user_token()))
        .await;

    // Assert
    let saved_user = sqlx::query!(
        "SELECT username, email, password FROM users WHERE id = $1",
        app.test_user().id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch saved user");

    assert_eq!("foo@bar.com", saved_user.email);
    assert_eq!("FooBar", saved_user.username);
    assert!(verify_password_hash(saved_user.password, "foobar123".to_string()).unwrap());
}
