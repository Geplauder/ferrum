use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use uuid::Uuid;

use crate::{
    jwt::AuthorizationService,
    websocket::{
        messages::{
            RegisterChannelsForUser, SerializedWebSocketMessage, WebSocketClose, WebSocketConnect,
            WebSocketMessage,
        },
        Server,
    },
};

struct WebSocketSession {
    user_id: Uuid,
    server: Addr<Server>,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();

        self.server
            .do_send(WebSocketConnect::new(self.user_id, address.recipient()));
    }
}

impl Handler<SerializedWebSocketMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: SerializedWebSocketMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.0);
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

                if let WebSocketMessage::Bootstrap(bootstrap) = message {
                    let address = ctx.address();

                    self.server.do_send(RegisterChannelsForUser {
                        user_id: self.user_id,
                        addr: address.recipient(),
                        channels: bootstrap.channels,
                    });
                }
            }
            Ok(ws::Message::Close(reason)) => {
                self.server.do_send(WebSocketClose::new(self.user_id));

                ctx.close(reason);
                ctx.stop();
            }
            Err(_) => {
                ctx.stop();
            }
            _ => (),
        }
    }
}

pub async fn websocket(
    request: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<Server>>,
    auth: AuthorizationService,
) -> Result<HttpResponse, actix_web::Error> {
    let response = ws::start(
        WebSocketSession {
            user_id: auth.claims.id,
            server: server.get_ref().clone(),
        },
        &request,
        stream,
    )?;

    Ok(response)
}
