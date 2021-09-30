use std::{net::TcpListener, time::Duration};

use actix::Actor;
use actix_cors::Cors;
use actix_web::{dev::Server, web, web::Data, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_actix_web::TracingLogger;

use crate::{
    jwt::Jwt,
    routes::{channels, health_check, login, register, servers, users, websocket},
    settings::{DatabaseSettings, Settings},
};

pub struct ApplicationBaseUrl(pub String);

pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    pub async fn build(settings: Settings) -> Result<Self, std::io::Error> {
        let db_pool = get_db_pool(&settings.database)
            .await
            .expect("Could not connect to database.");

        let address = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();

        let websocket_server = crate::websocket::Server::new(db_pool.clone());

        let server = run(
            listener,
            db_pool,
            settings.application.base_url,
            settings.application.jwt_secret,
            websocket_server,
        )?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn get_db_pool(settings: &DatabaseSettings) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .connect_timeout(Duration::from_secs(5))
        .connect_with(settings.with_db())
        .await
}

fn run(
    listener: TcpListener,
    db_pool: PgPool,
    base_url: String,
    jwt_secret: String,
    websocket_server: crate::websocket::Server,
) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let jwt = Data::new(Jwt::new(jwt_secret));
    let websocket_server = Data::new(websocket_server.start());

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(
                Cors::default()
                    .allow_any_header()
                    .allow_any_method()
                    .allow_any_origin(),
            )
            .app_data(db_pool.clone())
            .app_data(base_url.clone())
            .app_data(jwt.clone())
            .app_data(websocket_server.clone())
            .route("/ws", web::get().to(websocket))
            .route("/health_check", web::get().to(health_check))
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/users", web::get().to(users::current_user))
            .route("/users/servers", web::get().to(users::current_user_servers))
            .route("/servers", web::post().to(servers::create))
            .route("/servers/{id}", web::get().to(servers::get))
            .route("/servers/{id}", web::put().to(servers::join))
            .route("/servers/{id}/users", web::get().to(servers::get_users))
            .route(
                "/servers/{id}/channels",
                web::get().to(servers::get_channels),
            )
            .route(
                "/servers/{id}/channels",
                web::post().to(servers::create_channel),
            )
            .route(
                "/channels/{id}/messages",
                web::get().to(channels::get_messages),
            )
            .route(
                "/channels/{id}/messages",
                web::post().to(channels::create_message),
            )
    })
    .listen(listener)?
    .run();

    Ok(server)
}
