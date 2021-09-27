use crate::helpers::{spawn_app, BootstrapType};

#[actix_rt::test]
async fn health_check_works() {
    // Arrange
    let application = spawn_app(BootstrapType::Default).await;
    let client = awc::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &application.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
}
