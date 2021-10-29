use actix_http::{encoding::Decoder, Payload};
use ferrum_shared::server_invites::ServerInviteResponse;
use uuid::Uuid;

use crate::helpers::TestApplication;

impl TestApplication {
    pub async fn get_server_invites(
        &self,
        server_id: String,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .get(&format!("{}/servers/{}/invites", &self.address, server_id));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_invites_returns_200_for_valid_request() {
    // Arrange

    // Act
    let response = app
        .get_server_invites(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_invites_returns_valid_data_for_valid_request() {
    // Arrange

    // Act
    let response = app
        .get_server_invites(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await
        .json::<Vec<ServerInviteResponse>>()
        .await
        .unwrap();

    // Assert
    assert_eq!(1, response.len());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_invites_fails_if_there_is_a_database_error() {
    // Arrange

    sqlx::query!("ALTER TABLE server_invites DROP COLUMN code;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app
        .get_server_invites(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_invites_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app
            .get_server_invites(app.test_server().id.to_string(), token)
            .await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[ferrum_macros::test(strategy = "UserAndOtherServer")]
async fn get_invites_returns_403_when_user_does_not_have_access_to_server() {
    // Arrange

    // Act
    let response = app
        .get_server_invites(
            app.test_server().id.to_string(),
            Some(app.test_user_token()),
        )
        .await;

    // Assert
    assert_eq!(403, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_invites_returns_404_when_server_id_is_invalid() {
    // Arrange

    // Act
    let response = app
        .get_server_invites("foo".to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(404, response.status().as_u16());
}

#[ferrum_macros::test(strategy = "UserAndOwnServer")]
async fn get_invites_returns_500_when_server_id_is_not_found() {
    // Arrange

    // Act
    let response = app
        .get_server_invites(Uuid::new_v4().to_string(), Some(app.test_user_token()))
        .await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}
