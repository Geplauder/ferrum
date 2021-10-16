use std::collections::{HashMap, HashSet};

use ferrum_shared::jwt::Jwt;
use ferrum_websocket::{WebSocketServer, WebSocketSession};
use futures_util::StreamExt;
use meio::{Address, System};
use sqlx::postgres::PgPoolOptions;
use tokio::net::{TcpListener, TcpStream};
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
        jwt: Jwt::new("foobar".to_string()),
        server,
    });

    session.attach(incoming, ()).unwrap();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let pool = PgPoolOptions::new().connect("uri").await.unwrap();

    let address = System::spawn(WebSocketServer {
        users: HashMap::new(),
        db_pool: pool,
    });

    let addr = "127.0.0.1:8001";
    let listener = TcpListener::bind(&addr).await.unwrap();

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, address.clone()));
    }

    Ok(())
}
