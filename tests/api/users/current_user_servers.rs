use actix_http::{encoding::Decoder, Payload};
use ferrum::domain::servers::Server;

use crate::helpers::{spawn_app, BootstrapType, TestApplication};

impl TestApplication {
    pub async fn get_user_servers(
        &self,
        bearer: Option<String>,
    ) -> awc::ClientResponse<Decoder<Payload>> {
        let mut client = self
            .http_client()
            .get(&format!("{}/users/servers", &self.address));

        if let Some(bearer) = bearer {
            client = client.bearer_auth(bearer);
        }

        client.send().await.expect("Failed to execute request.")
    }
}

#[actix_rt::test]
async fn current_user_servers_returns_200_for_valid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Todo: Improve this
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    // Act
    let mut response = app.get_user_servers(Some(app.test_user_token())).await;

    // Assert
    assert_eq!(200, response.status().as_u16());

    let user_servers = response.json::<Vec<Server>>().await.unwrap();
    let user_server = user_servers.first().unwrap();

    assert_eq!(app.test_server().id, user_server.id);
    assert_eq!(app.test_server().name, user_server.name);
    assert_eq!(app.test_user().id, user_server.owner_id);
}

#[actix_rt::test]
async fn current_user_servers_returns_401_for_missing_or_invalid_bearer_token() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Todo: Improve this
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    for token in [None, Some("foo".to_string())] {
        // Act
        let response = app.get_user_servers(token).await;

        // Assert
        assert_eq!(401, response.status().as_u16());
    }
}

#[actix_rt::test]
async fn current_user_servers_fails_if_there_is_a_database_error() {
    // Arrange
    let app = spawn_app(BootstrapType::UserAndOwnServer).await;

    // Todo: Improve this
    app.put_join_server(
        app.test_server().id.to_string(),
        Some(app.test_user_token()),
    )
    .await;

    sqlx::query!("ALTER TABLE servers DROP COLUMN owner_id;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.get_user_servers(Some(app.test_user_token())).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}
