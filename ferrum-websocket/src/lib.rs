#![allow(clippy::bool_comparison)]

pub mod messages;
mod server;

use std::{collections::HashSet, iter::FromIterator};

use async_trait::async_trait;
use ferrum_shared::jwt::Jwt;
use futures_util::{stream::SplitSink, SinkExt};
use meio::{ActionHandler, Actor, Address, Consumer, Context, StartedBy, StreamAcceptor, System};
use messages::{IdentifyUser, SerializedWebSocketMessage, WebSocketClose, WebSocketSessionMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use uuid::Uuid;

pub use server::WebSocketServer;

type TungsteniteMessage =
    tokio_tungstenite::tungstenite::Result<tokio_tungstenite::tungstenite::Message>;

///
/// This contains data for the websocket connection for a specific user.
///
/// For each new websocket client, a new session will be created.
/// Due to that, reconnections are currently not supported.
///
/// When a websocket client closes the connection, the session will also be stopped and disposed.
///
pub struct WebSocketSession {
    pub connection: SplitSink<WebSocketStream<TcpStream>, Message>,
    pub user_id: Option<Uuid>,
    pub server: Address<WebSocketServer>,
    pub channels: HashSet<Uuid>,
    pub servers: HashSet<Uuid>,
    pub jwt: Jwt,
}

impl Actor for WebSocketSession {
    type GroupBy = ();
}

#[async_trait]
impl StartedBy<System> for WebSocketSession {
    async fn handle(&mut self, _ctx: &mut Context<Self>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

#[async_trait]
impl Consumer<TungsteniteMessage> for WebSocketSession {
    async fn handle(
        &mut self,
        message: TungsteniteMessage,
        ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        // Currently we're only handling (and sending) text messages.
        // In the future, we should probably move to binary messages to reduce overhead.
        match message {
            Ok(Message::Text(text)) => {
                let message = match serde_json::from_str::<SerializedWebSocketMessage>(&text) {
                    Ok(value) => value,
                    Err(_) => return Err(anyhow::anyhow!("todo")),
                };

                match message {
                    SerializedWebSocketMessage::Ping => {
                        // Respond to Ping with Pong
                        self.connection
                            .send(Message::Text(
                                serde_json::to_string(&SerializedWebSocketMessage::Pong).unwrap(),
                            ))
                            .await
                            .unwrap();
                    }
                    SerializedWebSocketMessage::Identify { bearer } => {
                        // Check if there are claims for the JWT, if so identify with the websocket server
                        let claims = match self.jwt.get_claims(&bearer) {
                            Some(value) => value,
                            None => return Err(anyhow::anyhow!("todo")),
                        };

                        let address = ctx.address();

                        self.user_id = Some(claims.id);
                        self.server
                            .act(IdentifyUser {
                                user_id: claims.id,
                                addr: address.clone(),
                            })
                            .await
                            .unwrap();
                    }
                    _ => (),
                }
            }
            Ok(Message::Close(_reason)) => {
                // If the user was identified, notify the websocket server about the closed session
                if let Some(user_id) = self.user_id {
                    self.server.act(WebSocketClose::new(user_id)).await.unwrap();
                }

                self.connection.close().await.unwrap();
                ctx.stop();
            }
            _ => (),
        }

        Ok(())
    }

    async fn finished(&mut self, ctx: &mut Context<Self>) -> Result<(), anyhow::Error> {
        ctx.shutdown();

        Ok(())
    }
}

impl StreamAcceptor<TungsteniteMessage> for WebSocketSession {
    fn stream_group(&self) -> Self::GroupBy {}
}

#[async_trait]
impl ActionHandler<WebSocketSessionMessage> for WebSocketSession {
    async fn handle(
        &mut self,
        msg: WebSocketSessionMessage,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        match msg {
            WebSocketSessionMessage::Ready(servers, channels) => {
                // Store the servers and channels and inform the client that it is now ready
                self.servers = HashSet::from_iter(servers.iter().cloned());
                self.channels = HashSet::from_iter(channels.iter().cloned());

                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::Ready).unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::AddMessage(message) => {
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewMessage { message })
                            .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::AddChannel(channel) => {
                // Store the new channel and send it to the client
                self.channels.insert(channel.id);

                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewChannel { channel })
                            .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::AddServer(server, channels, users) => {
                // Store the new server (and channel) and sent it to the client
                self.servers.insert(server.id);
                self.channels.extend(channels.iter().map(|x| x.id));

                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewServer {
                            server,
                            channels,
                            users,
                        })
                        .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::AddUser(server_id, user) => {
                // Send the new user to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewUser {
                            server_id,
                            user,
                        })
                        .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::DeleteUser(user_id, server_id) => {
                // Send the deleted/leaving user to the client

                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::DeleteUser {
                            user_id,
                            server_id,
                        })
                        .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::DeleteServer(server_id) => {
                // Check if the user is part of the server, if so remove the server and sent the removed server to the client
                if self.servers.contains(&server_id) == false {
                    return Err(anyhow::anyhow!("todo"));
                }

                self.servers.remove(&server_id);

                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::DeleteServer {
                            server_id,
                        })
                        .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
            WebSocketSessionMessage::UpdateServer(server) => {
                // Send the updated server to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::UpdateServer { server })
                            .unwrap(),
                    ))
                    .await
                    .unwrap();
            }
        }

        Ok(())
    }
}
