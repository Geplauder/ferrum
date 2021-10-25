#![allow(clippy::bool_comparison)]

pub mod application;
pub mod messages;
mod server;

use std::{collections::HashSet, iter::FromIterator};

use anyhow::Context as AnyhowContext;
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
    #[tracing::instrument(name = "Handle new incoming websocket message", skip(self, ctx), fields(request_id = %Uuid::new_v4(), user_id = ?self.user_id))]
    async fn handle(
        &mut self,
        message: TungsteniteMessage,
        ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        // Currently we're only handling (and sending) text messages.
        // In the future, we should probably move to binary messages to reduce overhead.
        match message {
            Ok(Message::Text(text)) => {
                let message = serde_json::from_str::<SerializedWebSocketMessage>(&text)
                    .context("Failed to deserialize websocket message")?;

                match message {
                    SerializedWebSocketMessage::Ping => {
                        // Respond to Ping with Pong
                        self.connection
                            .send(Message::Text(
                                serde_json::to_string(&SerializedWebSocketMessage::Pong)
                                    .context("Failed to serialize Pong websocket message")?,
                            ))
                            .await
                            .context("Failed to send Pong websocket message")?;
                    }
                    SerializedWebSocketMessage::Identify { bearer } => {
                        // Check if there are claims for the JWT, if so identify with the websocket server
                        let claims = self
                            .jwt
                            .get_claims(&bearer)
                            .context("Failed to get claims for bearer token")?;

                        let address = ctx.address();

                        self.user_id = Some(claims.id);
                        self.server
                            .act(IdentifyUser {
                                user_id: claims.id,
                                addr: address.clone(),
                            })
                            .await
                            .context("Failed to send IdentifyUser message to websocket server")?;
                    }
                    _ => (),
                }
            }
            Ok(Message::Close(_reason)) => {
                // If the user was identified, notify the websocket server about the closed session
                if let Some(user_id) = self.user_id {
                    self.server
                        .act(WebSocketClose::new(user_id))
                        .await
                        .context("Failed to send WebSocketClose message to websocket server")?;
                }

                self.connection
                    .close()
                    .await
                    .context("Failed to close websocket stream")?;
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
    #[tracing::instrument(name = "Handle outgoing websocket message", skip(self, _ctx), fields(request_id = %Uuid::new_v4(), user_id = ?self.user_id))]
    async fn handle(
        &mut self,
        msg: WebSocketSessionMessage,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        match msg {
            WebSocketSessionMessage::Ready(servers) => {
                // Store the servers and inform the client that it is now ready
                self.servers = HashSet::from_iter(servers.iter().cloned());

                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::Ready)
                            .context("Failed to serialize Ready websocket message")?,
                    ))
                    .await
                    .context("Failed to send Rady websocket message")?;
            }
            WebSocketSessionMessage::AddMessage(message) => {
                // Send the new message to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewMessage { message })
                            .context("Failed to serialize NewMessage websocket message")?,
                    ))
                    .await
                    .context("Failed to send NewMessage websocket message")?;
            }
            WebSocketSessionMessage::AddChannel(channel) => {
                // Send the new channel to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewChannel { channel })
                            .context("Failed to serialize NewChannel websocket message")?,
                    ))
                    .await
                    .context("Failed to send AddChannel websocket message")?;
            }
            WebSocketSessionMessage::AddServer(server, channels, users) => {
                // Store the new server and sent it to the client
                self.servers.insert(server.id);
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewServer {
                            server,
                            channels,
                            users,
                        })
                        .context("Failed to serialize NewServer websocket message")?,
                    ))
                    .await
                    .context("Failed to send AddServer websocket message")?;
            }
            WebSocketSessionMessage::AddUser(server_id, user) => {
                // Send the new user to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::NewUser {
                            server_id,
                            user,
                        })
                        .context("Failed to serialize NewUser websocket message")?,
                    ))
                    .await
                    .context("Failed to send NewUser websocket message")?;
            }
            WebSocketSessionMessage::DeleteUser(user_id, server_id) => {
                // Send the deleted/leaving user to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::DeleteUser {
                            user_id,
                            server_id,
                        })
                        .context("Failed to serialize DeleteUser websocket message")?,
                    ))
                    .await
                    .context("Failed to send DeleteUser websocket message")?;
            }
            WebSocketSessionMessage::DeleteServer(server_id) => {
                // Try to remove the server from the users' servers, if it was successful notify the client about it
                if self.servers.remove(&server_id) {
                    self.connection
                        .send(Message::Text(
                            serde_json::to_string(&SerializedWebSocketMessage::DeleteServer {
                                server_id,
                            })
                            .context("Failed to serialize DeleteServer websocket message")?,
                        ))
                        .await
                        .context("Failed to send DeleteServer websocket message")?;
                }
            }
            WebSocketSessionMessage::DeleteChannel(channel_id) => {
                // Send the deleted channel to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::DeleteChannel {
                            channel_id,
                        })
                        .context("Failed to serialize DeleteChannel websocket message")?,
                    ))
                    .await
                    .context("Failed to send DeleteChannel websocket message")?;
            }
            WebSocketSessionMessage::UpdateServer(server) => {
                // Send the updated server to the client
                self.connection
                    .send(Message::Text(
                        serde_json::to_string(&SerializedWebSocketMessage::UpdateServer { server })
                            .context("Failed to serialize UpdateServer websocket message")?,
                    ))
                    .await
                    .context("Failed to send UpdateServer websocket message")?;
            }
        }

        Ok(())
    }
}
