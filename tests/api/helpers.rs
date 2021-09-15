use ferrum::{
    application::{get_db_pool, Application},
    settings::{get_settings, DatabaseSettings},
    telemetry::{get_subscriber, init_subscriber},
};
use once_cell::sync::Lazy;
use sqlx::{types::Uuid, Connection, Executor, PgConnection, PgPool};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber)
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber)
    };
});

pub struct TestApplication {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
}

impl TestApplication {
    pub async fn post_register(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/register", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub async fn spawn_app() -> TestApplication {
    Lazy::force(&TRACING);

    let settings = {
        let mut settings = get_settings().expect("Failed to read settings");

        settings.database.database_name = Uuid::new_v4().to_string();
        settings.application.port = 0;

        settings
    };

    configure_database(&settings.database).await;

    let application = Application::build(settings.clone())
        .await
        .expect("Failed to build application");

    let application_port = application.port();

    let _ = tokio::spawn(application.run_until_stopped());

    TestApplication {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool: get_db_pool(&settings.database)
            .await
            .expect("Failed to connect to database"),
    }
}

async fn configure_database(settings: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(&*format!(
            r#"CREATE DATABASE "{}";"#,
            settings.database_name
        ))
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(settings.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
