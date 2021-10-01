use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use uuid::Uuid;

use crate::{
    jwt::Jwt,
    websocket::{
        messages::{IdentifyUser, SerializedWebSocketMessage, WebSocketClose, WebSocketMessage},
        Server,
    },
};

pub struct WebSocketSession {
    pub user_id: Option<Uuid>,
    pub server: Addr<Server>,
    pub channels: Vec<Uuid>,
    jwt: Jwt,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;
}

impl Handler<SerializedWebSocketMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: SerializedWebSocketMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SerializedWebSocketMessage::Ready(channels) => {
                self.channels = channels;

                ctx.text(serde_json::to_string(&WebSocketMessage::Ready).unwrap());
            }
            SerializedWebSocketMessage::Data(data, channel) => {
                if self.channels.contains(&channel) {
                    ctx.text(data);
                }
            }
            SerializedWebSocketMessage::AddChannel(channel) => {
                self.channels.push(channel.id);

                ctx.text(serde_json::to_string(&WebSocketMessage::NewChannel { channel }).unwrap());
            }
            SerializedWebSocketMessage::AddServer(server, channels) => {
                self.channels.extend(channels.iter().map(|x| x.id));

                ctx.text(serde_json::to_string(&WebSocketMessage::NewServer { server }).unwrap());
            }
            SerializedWebSocketMessage::AddUser(user) => {
                ctx.text(serde_json::to_string(&WebSocketMessage::NewUser { user }).unwrap());
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => {
                let message = match serde_json::from_str::<WebSocketMessage>(&text) {
                    Ok(value) => value,
                    Err(_) => return,
                };

                match message {
                    WebSocketMessage::Ping => {
                        ctx.text(serde_json::to_string(&WebSocketMessage::Pong).unwrap());
                    }
                    WebSocketMessage::Identify { bearer } => {
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
    server: web::Data<Addr<Server>>,
) -> Result<HttpResponse, actix_web::Error> {
    let response = ws::start(
        WebSocketSession {
            user_id: None,
            server: server.get_ref().clone(),
            channels: vec![],
            jwt: jwt.as_ref().clone(),
        },
        &request,
        stream,
    )?;

    Ok(response)
}
