use actix_http::{encoding::Decoder, Payload};
use uuid::Uuid;

use crate::helpers::TestApplication;

impl TestApplication {
    pub async fn delete_server(
        &self,
        server_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .delete(&format!("{}/servers/{}", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_server_returns_200_for_valid_request() {
    // Arrange

    // Act
    let response = app
        .delete_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_server_deletes_the_server_from_the_database() {
    // Arrange

    // Act
    app.delete_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Assert
    let saved_servers = sqlx::query!("SELECT name, owner_id FROM servers")
        .fetch_all(&app.db_pool)
        .await
        .expect("Failed to fetch server count");

    assert!(saved_servers.is_empty());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_server_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .delete_server(app.test_server().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn delete_server_returns_403_for_users_without_access() {
    // Arrange

    // Act
    let response = app
        .delete_server(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_server_returns_404_when_server_id_is_invalid() {
    // Arrange

    // Act
    let response = app
        .delete_server("foobar".to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn delete_server_returns_500_when_server_id_is_not_found() {
    // Arrange

    // Act
    let response = app
        .delete_server(Uuid::new_v4().to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}
