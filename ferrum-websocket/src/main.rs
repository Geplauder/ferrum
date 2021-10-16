use std::collections::{HashMap, HashSet};

use ferrum_shared::{jwt::Jwt, settings::get_settings};
use ferrum_websocket::{messages::BrokerEvent, WebSocketServer, WebSocketSession};
use futures_util::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use meio::{Address, System};
use sqlx::postgres::PgPoolOptions;
use tokio::net::{TcpListener, TcpStream};
use tokio_amqp::LapinTokioExt;
use tokio_tungstenite::accept_async;

async fn handle_connection(
    stream: TcpStream,
    server: Address<WebSocketServer>,
) -> tokio_tungstenite::tungstenite::Result<()> {
    let websocket_stream = accept_async(stream).await.unwrap();

    let (outgoing, incoming) = websocket_stream.split();

    let mut session = System::spawn(WebSocketSession {
        connection: outgoing,
        user_id: None,
        channels: HashSet::new(),
        servers: HashSet::new(),
        jwt: Jwt::new("foo".to_string()),
        server,
    });

    session.attach(incoming, ()).unwrap();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let settings = get_settings().expect("Failed to read settings");

    let pool = PgPoolOptions::new()
        .connect_with(settings.database.with_db())
        .await
        .unwrap();

    let address = System::spawn(WebSocketServer {
        users: HashMap::new(),
        db_pool: pool,
    });

    let address_clone = address.clone();

    let addr = "127.0.0.1:8001";
    let listener = TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        let connection = Connection::connect(
            &settings.broker.get_connection_string(),
            ConnectionProperties::default().with_tokio(),
        )
        .await
        .unwrap();

        let channel = connection.create_channel().await.unwrap();

        let mut consumer = channel
            .basic_consume(
                &settings.broker.queue,
                "websocket_server",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        while let Some(message) = consumer.next().await {
            let (_, delivery) = message.unwrap();

            delivery.ack(BasicAckOptions::default()).await.unwrap();

            let broker_event: BrokerEvent = serde_json::from_slice(&delivery.data).unwrap();

            address_clone.clone().act(broker_event).await.unwrap();
        }
    });

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, address.clone()));
    }

    Ok(())
}
