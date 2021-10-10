#![allow(clippy::bool_comparison)]

pub mod messages;
mod server;

use std::{
    collections::{HashSet, VecDeque},
    iter::FromIterator,
};

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use ferrum_shared::jwt::Jwt;
use messages::{IdentifyUser, SerializedWebSocketMessage, WebSocketClose, WebSocketMessageType};
use uuid::Uuid;

pub use server::WebSocketServer;

macro_rules! save_and_send_message {
    ($self:expr, $ctx:expr, $message:expr) => {
        let last_index = $self.messages.back().map(|x| x.0);

        let index = if let Some(index) = last_index {
            $self.messages.push_back((index + 1, $message.clone()));

            index + 1
        } else {
            $self.messages.push_back((0, $message.clone()));

            0
        };

        $ctx.text(
            ::serde_json::to_string(&crate::messages::WebSocketMessage {
                id: index,
                payload: $message,
            })
            .unwrap(),
        );
    };
}

pub struct WebSocketSession {
    pub user_id: Option<Uuid>,
    pub server: Addr<WebSocketServer>,
    pub channels: HashSet<Uuid>,
    pub servers: HashSet<Uuid>,
    pub messages: VecDeque<(u64, WebSocketMessageType)>,
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
                self.servers = HashSet::from_iter(servers.iter().cloned());
                self.channels = HashSet::from_iter(channels.iter().cloned());

                save_and_send_message!(self, ctx, WebSocketMessageType::Ready);
            }
            SerializedWebSocketMessage::Data(message, channel) => {
                if self.channels.contains(&channel) {
                    save_and_send_message!(self, ctx, message);
                }
            }
            SerializedWebSocketMessage::AddChannel(channel) => {
                self.channels.insert(channel.id);

                let message = WebSocketMessageType::NewChannel { channel };
                save_and_send_message!(self, ctx, message);
            }
            SerializedWebSocketMessage::AddServer(server, channels, users) => {
                self.servers.insert(server.id);
                self.channels.extend(channels.iter().map(|x| x.id));

                let message = WebSocketMessageType::NewServer {
                    server,
                    channels,
                    users,
                };

                save_and_send_message!(self, ctx, message);
            }
            SerializedWebSocketMessage::AddUser(server_id, user) => {
                if self.servers.contains(&server_id) == false {
                    return;
                }

                let message = WebSocketMessageType::NewUser { server_id, user };
                save_and_send_message!(self, ctx, message);
            }
            SerializedWebSocketMessage::DeleteUser(user_id, server_id) => {
                let message = WebSocketMessageType::DeleteUser { user_id, server_id };
                save_and_send_message!(self, ctx, message);
            }
            SerializedWebSocketMessage::DeleteServer(server_id) => {
                if self.servers.contains(&server_id) == false {
                    return;
                }

                self.servers.remove(&server_id);

                let message = WebSocketMessageType::DeleteServer { server_id };
                save_and_send_message!(self, ctx, message);
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => {
                let message = match serde_json::from_str::<WebSocketMessageType>(&text) {
                    Ok(value) => value,
                    Err(_) => return,
                };

                match message {
                    WebSocketMessageType::Ping => {
                        ctx.text(serde_json::to_string(&WebSocketMessageType::Pong).unwrap());
                    }
                    WebSocketMessageType::Identify { bearer } => {
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
            messages: VecDeque::new(),
            jwt: jwt.as_ref().clone(),
        },
        &request,
        stream,
    )?;

    Ok(response)
}
