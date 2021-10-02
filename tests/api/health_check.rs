#[ferrum_macros::test]
async fn health_check_works() {
    // Arrange
    let client = awc::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
}
