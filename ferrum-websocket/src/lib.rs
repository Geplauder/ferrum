#![allow(clippy::bool_comparison)]

pub mod messages;
mod server;

use std::{collections::HashSet, iter::FromIterator};

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use ferrum_shared::jwt::Jwt;
use messages::{IdentifyUser, SerializedWebSocketMessage, WebSocketClose, WebSocketMessage};
use uuid::Uuid;

pub use server::WebSocketServer;

///
/// This contains data for the websocket connection for a specific user.
///
/// For each new websocket client, a new session will be created.
/// Due to that, reconnections are currently not supported.
///
/// When a websocket client closes the connection, the session will also be stopped and disposed.
///
pub struct WebSocketSession {
    pub user_id: Option<Uuid>,
    pub server: Addr<WebSocketServer>,
    pub channels: HashSet<Uuid>,
    pub servers: HashSet<Uuid>,
    jwt: Jwt,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;
}

impl Handler<SerializedWebSocketMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: SerializedWebSocketMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SerializedWebSocketMessage::Ready(servers, channels) => {
                // Store the servers and channels and inform the client that it is now ready
                self.servers = HashSet::from_iter(servers.iter().cloned());
                self.channels = HashSet::from_iter(channels.iter().cloned());

                ctx.text(serde_json::to_string(&WebSocketMessage::Ready).unwrap());
            }
            SerializedWebSocketMessage::Data(data, channel) => {
                // If the user is part of the channel, send the raw data
                if self.channels.contains(&channel) {
                    ctx.text(data);
                }
            }
            SerializedWebSocketMessage::AddChannel(channel) => {
                // Store the new channel and send it to the client
                self.channels.insert(channel.id);

                ctx.text(serde_json::to_string(&WebSocketMessage::NewChannel { channel }).unwrap());
            }
            SerializedWebSocketMessage::AddServer(server, channels, users) => {
                // Store the new server (and channel) and sent it to the client
                self.servers.insert(server.id);
                self.channels.extend(channels.iter().map(|x| x.id));

                ctx.text(
                    serde_json::to_string(&WebSocketMessage::NewServer {
                        server,
                        channels,
                        users,
                    })
                    .unwrap(),
                );
            }
            SerializedWebSocketMessage::AddUser(server_id, user) => {
                // Check if the user is part of the server, if so send the new user to the client
                if self.servers.contains(&server_id) == false {
                    return;
                }

                ctx.text(
                    serde_json::to_string(&WebSocketMessage::NewUser { server_id, user }).unwrap(),
                );
            }
            SerializedWebSocketMessage::DeleteUser(user_id, server_id) => {
                // Send the deleted/leaving user to the client

                ctx.text(
                    serde_json::to_string(&WebSocketMessage::DeleteUser { user_id, server_id })
                        .unwrap(),
                );
            }
            SerializedWebSocketMessage::DeleteServer(server_id) => {
                // Check if the user is part of the server, if so remove the server and sent the removed server to the client
                if self.servers.contains(&server_id) == false {
                    return;
                }

                self.servers.remove(&server_id);

                ctx.text(
                    serde_json::to_string(&WebSocketMessage::DeleteServer { server_id }).unwrap(),
                )
            }
            SerializedWebSocketMessage::UpdateServer(server) => {
                // Send the updated server to the client

                ctx.text(
                    serde_json::to_string(&WebSocketMessage::UpdateServer { server }).unwrap(),
                );
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // Currently we're only handling (and sending) text messages.
        // In the future, we should probably move to binary messages to reduce overhead.
        match item {
            Ok(ws::Message::Text(text)) => {
                let message = match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(value) => value,
                    Err(_) => return,
                };

                match message {
                    WebSocketMessage::Ping => {
                        // Respond to Ping with Pong
                        ctx.text(serde_json::to_string(&WebSocketMessage::Pong).unwrap());
                    }
                    WebSocketMessage::Identify { bearer } => {
                        // Check if there are claims for the JWT, if so identify with the websocket server
                        let claims = match self.jwt.get_claims(&bearer) {
                            Some(value) => value,
                            None => return,
                        };

                        let address = ctx.address();

                        self.user_id = Some(claims.id);
                        self.server.do_send(IdentifyUser {
                            user_id: claims.id,
                            addr: address.recipient(),
                        });
                    }
                    _ => (),
                }
            }
            Ok(ws::Message::Close(reason)) => {
                // If the user was identified, notify the websocket server about the closed session
                if let Some(user_id) = self.user_id {
                    self.server.do_send(WebSocketClose::new(user_id));
                }

                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}

pub async fn websocket(
    request: HttpRequest,
    stream: web::Payload,
    jwt: web::Data<Jwt>,
    server: web::Data<Addr<WebSocketServer>>,
) -> Result<HttpResponse, actix_web::Error> {
    let response = ws::start(
        WebSocketSession {
            user_id: None,
            server: server.get_ref().clone(),
            channels: HashSet::new(),
            servers: HashSet::new(),
            jwt: jwt.as_ref().clone(),
        },
        &request,
        stream,
    )?;

    Ok(response)
}
