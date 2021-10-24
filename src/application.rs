use std::net::TcpListener;

use actix::Actor;
use actix_cors::Cors;
use actix_web::{dev::Server, web, web::Data, App, HttpServer};
use ferrum_shared::{
    jwt::Jwt,
    settings::{get_db_pool, Settings},
};
use lapin::{Connection, ConnectionProperties};
use sqlx::PgPool;
use tokio_amqp::LapinTokioExt;
use tracing_actix_web::TracingLogger;

use crate::{
    broker::Broker,
    routes::{channels, health_check, login, register, servers, users},
};

pub struct ApplicationBaseUrl(pub String);

pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    ///
    /// Build a instance of [`Application`] from the supplied [`Settings`].
    ///
    /// This also creates a database pool, binds a address for the web server
    /// and starts the websocket server.
    ///
    pub async fn build(settings: Settings) -> Result<Self, std::io::Error> {
        let db_pool = get_db_pool(&settings.database)
            .await
            .expect("Could not connect to database.");

        let address = format!(
            "{}:{}",
            settings.application.host, settings.application.port
        );

        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr()?.port();

        let ampq_connection = Connection::connect(
            &settings.broker.get_connection_string(),
            ConnectionProperties::default().with_tokio(),
        )
        .await
        .unwrap();

        let channel = ampq_connection.create_channel().await.unwrap();

        let broker = Broker {
            queue: settings.broker.queue,
            channel,
        };

        let server = run(
            listener,
            db_pool,
            settings.application.base_url,
            settings.application.jwt_secret,
            broker,
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

fn run(
    listener: TcpListener,
    db_pool: PgPool,
    base_url: String,
    jwt_secret: String,
    broker: Broker,
) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let jwt = Data::new(Jwt::new(jwt_secret));
    let broker = Data::new(broker.start());

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
            .app_data(broker.clone())
            .route("/health_check", web::get().to(health_check))
            .route("/register", web::post().to(register))
            .route("/login", web::post().to(login))
            .route("/users", web::get().to(users::current_user))
            .route("/users", web::post().to(users::update))
            .route("/users/servers", web::get().to(users::current_user_servers))
            .route("/servers", web::post().to(servers::create))
            .route("/servers/{id}", web::get().to(servers::get))
            .route("/servers/{id}", web::post().to(servers::update))
            .route("/servers/{id}", web::put().to(servers::join))
            .route("/servers/{id}", web::delete().to(servers::delete))
            .route("/servers/{id}/users", web::get().to(servers::get_users))
            .route("/servers/{id}/users", web::delete().to(servers::leave))
            .route(
                "/servers/{id}/channels",
                web::get().to(servers::get_channels),
            )
            .route(
                "/servers/{id}/channels",
                web::post().to(servers::create_channel),
            )
            .route("/channels/{id}", web::delete().to(channels::delete))
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
