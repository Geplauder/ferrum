use crate::helpers::spawn_app;

#[actix_rt::test]
async fn register_returns_200_for_valid_json_data() {
    // Arrange
    let app = spawn_app().await;
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