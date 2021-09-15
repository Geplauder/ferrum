use std::{net::TcpListener, time::Duration};

use actix_web::{dev::Server, web, web::Data, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_actix_web::TracingLogger;

use crate::{
    jwt::Jwt,
    routes::{health_check, login, register},
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

        let server = run(
            listener,
            db_pool,
            settings.application.base_url,
            settings.application.jwt_secret,
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
) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let jwt = Data::new(Jwt::new(jwt_secret));

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .app_data(db_pool.clone())
            .app_data(base_url.clone())
            .app_data(jwt.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
