use std::time::Duration;

use actix_http::{client::ConnectionIo, ws};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use fake::Fake;
use ferrum::{
    application::{get_db_pool, Application},
    settings::{get_settings, DatabaseSettings, Settings},
    telemetry::{get_subscriber, init_subscriber},
};
use ferrum_shared::jwt::Jwt;
use ferrum_websocket::messages::WebSocketMessage;
use futures::{select, FutureExt, SinkExt, StreamExt};
use once_cell::sync::Lazy;
use sqlx::{postgres::PgPoolOptions, types::Uuid, Connection, Executor, PgConnection, PgPool};

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

pub enum BootstrapType {
    Default,
    User,
    UserAndOwnServer,
    UserAndOtherServer,
}

pub struct TestApplication {
    pub address: String,
    pub ws_address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub jwt: Jwt,
    pub settings: Settings,
    test_user: Option<TestUser>,
    test_user_token: Option<String>,
    test_server: Option<TestServer>,
}

impl TestApplication {
    pub fn http_client(&self) -> awc::Client {
        awc::Client::builder()
            .timeout(Duration::from_secs(15))
            .finish()
    }

    pub fn test_user(&self) -> TestUser {
        self.test_user.as_ref().unwrap().clone()
    }

    pub fn test_user_token(&self) -> String {
        self.test_user_token.as_ref().unwrap().clone()
    }

    pub fn test_server(&self) -> TestServer {
        self.test_server.as_ref().unwrap().clone()
    }

    pub async fn websocket(
        &self,
    ) -> (
        awc::ClientResponse,
        actix_codec::Framed<Box<dyn ConnectionIo>, actix_http::ws::Codec>,
    ) {
        self.http_client()
            .ws(&format!("{}/ws", &self.ws_address))
            .connect()
            .await
            .unwrap()
    }
}

pub async fn spawn_app(bootstrap_type: BootstrapType) -> TestApplication {
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

    let mut test_application = TestApplication {
        address: format!("http://localhost:{}", application_port),
        ws_address: format!("ws://localhost:{}", application_port),
        port: application_port,
        db_pool: get_db_pool(&settings.database)
            .await
            .expect("Failed to connect to database"),
        jwt: Jwt::new(settings.application.jwt_secret.to_owned()),
        settings: settings,
        test_user_token: None,
        test_user: None,
        test_server: None,
    };

    match bootstrap_type {
        BootstrapType::Default => (),
        BootstrapType::User => {
            let test_user = TestUser::generate();
            test_user.store(&test_application.db_pool).await;

            test_application.test_user_token = Some(
                test_application
                    .jwt
                    .encode(test_user.id.to_owned(), test_user.email.to_owned()),
            );
            test_application.test_user = Some(test_user);
        }
        BootstrapType::UserAndOwnServer => {
            let test_user = TestUser::generate();
            test_user.store(&test_application.db_pool).await;

            let test_server = TestServer::generate(test_user.id);
            test_server.store(&test_application.db_pool).await;

            test_application.test_user_token = Some(
                test_application
                    .jwt
                    .encode(test_user.id.to_owned(), test_user.email.to_owned()),
            );
            test_application.test_user = Some(test_user);
            test_application.test_server = Some(test_server);
        }
        BootstrapType::UserAndOtherServer => {
            let test_user = TestUser::generate();
            test_user.store(&test_application.db_pool).await;

            let dummy_user = TestUser::generate();
            dummy_user.store(&test_application.db_pool).await;
            let test_server = TestServer::generate(dummy_user.id);
            test_server.store(&test_application.db_pool).await;

            test_application.test_user_token = Some(
                test_application
                    .jwt
                    .encode(test_user.id.to_owned(), test_user.email.to_owned()),
            );
            test_application.test_user = Some(test_user);
            test_application.test_server = Some(test_server);
        }
    }

    test_application
}

pub async fn teardown(settings: &DatabaseSettings) {
    let mut connection = PgConnection::connect_with(&settings.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(&*format!(
            r#"
            SELECT pg_terminate_backend(pg_stat_activity.pid)
            FROM pg_stat_activity
            WHERE datname = '{}'
            AND pid <> pg_backend_pid();
            "#,
            settings.database_name
        ))
        .await
        .expect("Failed to delete database.");

    connection
        .execute(&*format!(
            "DROP DATABASE \"{}\" WITH (FORCE)",
            settings.database_name
        ))
        .await
        .expect("Failed to delete database.");
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

    let connection_pool = PgPoolOptions::new()
        .max_connections(1000)
        .connect_with(settings.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

pub async fn get_next_websocket_message(
    connection: &mut actix_codec::Framed<Box<dyn ConnectionIo>, actix_http::ws::Codec>,
) -> Option<WebSocketMessage> {
    let mut message = connection.next().fuse();
    let mut timeout = Box::pin(actix_rt::time::sleep(Duration::from_secs(2)).fuse());

    let x = select! {
        x = message => x,
        () = timeout => return None,
    };

    match x.unwrap().unwrap() {
        ws::Frame::Text(text) => match serde_json::from_slice::<WebSocketMessage>(&text) {
            Ok(value) => Some(value),
            Err(_) => None,
        },
        _ => None,
    }
}

pub async fn send_websocket_message(
    connection: &mut actix_codec::Framed<Box<dyn ConnectionIo>, actix_http::ws::Codec>,
    message: WebSocketMessage,
) {
    connection
        .send(ws::Message::Text(
            serde_json::to_string(&message).unwrap().into(),
        ))
        .await
        .unwrap();
}

#[derive(Debug, Clone)]
pub struct TestUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            email: fake::faker::internet::en::SafeEmail().fake(),
        }
    }

    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());

        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        sqlx::query!(
            "INSERT INTO users (id, username, email, password) VALUES ($1, $2, $3, $4)",
            self.id,
            self.name,
            self.email,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

#[derive(Debug, Clone)]
pub struct TestServer {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub default_channel_id: Uuid,
}

impl TestServer {
    pub fn generate(owner_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: Uuid::new_v4().to_string(),
            owner_id,
            default_channel_id: Uuid::new_v4(),
        }
    }

    pub async fn add_user(&self, user_id: Uuid, pool: &PgPool) {
        sqlx::query!(
            "INSERT INTO users_servers (id, user_id, server_id) VALUES ($1, $2, $3)",
            Uuid::new_v4(),
            user_id,
            self.id,
        )
        .execute(pool)
        .await
        .expect("Failed to store test server owner relation.");
    }

    pub async fn add_channel(&self, id: Uuid, channel_name: &str, pool: &PgPool) {
        sqlx::query!(
            "INSERT INTO channels (id, server_id, name) VALUES ($1, $2, $3)",
            id,
            self.id,
            channel_name,
        )
        .execute(pool)
        .await
        .expect("Failed to store server channel.");
    }

    async fn store(&self, pool: &PgPool) {
        sqlx::query!(
            "INSERT INTO servers (id, name, owner_id) VALUES ($1, $2, $3)",
            self.id,
            self.name,
            self.owner_id
        )
        .execute(pool)
        .await
        .expect("Failed to store test server.");

        self.add_channel(self.default_channel_id, "general", pool)
            .await;
        self.add_user(self.owner_id, pool).await;
    }
}
