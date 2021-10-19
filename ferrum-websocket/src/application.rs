use std::collections::HashSet;

use ferrum_shared::{
    broker::BrokerEvent,
    jwt::Jwt,
    settings::{get_db_pool, Settings},
};
use futures_util::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use meio::{Address, System};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;

use crate::{WebSocketServer, WebSocketSession};

pub struct Application {
    listener: TcpListener,
    ampq_connection: Connection,
    port: u16,
    jwt_secret: String,
    broker_queue: String,
    websocket_server: Address<WebSocketServer>,
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

        let websocket_server = System::spawn(WebSocketServer::new(db_pool));

        let ampq_connection = Connection::connect(
            &settings.broker.get_connection_string(),
            ConnectionProperties::default(),
        )
        .await
        .expect("Could not connect to broker");

        let listener = TcpListener::bind(&address).await?;
        let port = listener.local_addr()?.port();

        Ok(Self {
            listener,
            ampq_connection,
            port,
            jwt_secret: settings.application.jwt_secret,
            broker_queue: settings.broker.queue,
            websocket_server,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) {
        let ampq_connection = self.ampq_connection;
        let broker_queue = self.broker_queue;
        let address = self.websocket_server.clone();

        tokio::spawn(async move {
            let channel = ampq_connection.create_channel().await.unwrap();

            let mut consumer = channel
                .basic_consume(
                    &broker_queue,
                    "websocket_server",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .unwrap();

            while let Some(message) = consumer.next().await {
                tracing::info_span!("message broker handler", request_id = %uuid::Uuid::new_v4());

                let (_, delivery) = message.unwrap();

                delivery.ack(BasicAckOptions::default()).await.unwrap();

                let broker_event: BrokerEvent = serde_json::from_slice(&delivery.data).unwrap();

                if let Err(error) = address.clone().act(broker_event).await {
                    tracing::error!("Error while handling broker_event: {:?}", error);
                }
            }
        });

        while let Ok((stream, _)) = self.listener.accept().await {
            tokio::spawn(handle_connection(
                stream,
                self.websocket_server.clone(),
                self.jwt_secret.clone(),
            ));
        }
    }
}

#[tracing::instrument(name = "Handle a new incoming connection", skip(stream, server, jwt_secret), fields(request_id = %uuid::Uuid::new_v4()))]
async fn handle_connection(
    stream: TcpStream,
    server: Address<WebSocketServer>,
    jwt_secret: String,
) -> tokio_tungstenite::tungstenite::Result<()> {
    let websocket_stream = accept_async(stream).await.unwrap();

    let (outgoing, incoming) = websocket_stream.split();

    // We're creating a new WebSocketSession for each client that connects.
    // In the future, we may want to support reconnecting, for which the behaviour has to change.
    let mut session = System::spawn(WebSocketSession {
        connection: outgoing,
        user_id: None,
        servers: HashSet::new(),
        jwt: Jwt::new(jwt_secret),
        server,
    });

    session.attach(incoming, ()).unwrap();

    Ok(())
}
