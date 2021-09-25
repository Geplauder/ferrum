use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    time::Duration,
};

use actix::{Actor, AsyncContext, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::jwt::AuthorizationService;

pub static CHANNELS: Lazy<Mutex<WebSocketTable>> = Lazy::new(|| Mutex::new(WebSocketTable::new()));

pub struct WebSocketTable {
    map: HashMap<Uuid, Sender<String>>,
}

impl WebSocketTable {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    fn add_user(&mut self, user: Uuid, sender: Sender<String>) {
        self.map.insert(user, sender);
    }

    fn remove_user(&mut self, user: Uuid) {
        self.map.remove(&user);
    }

    pub fn send(&self, users: &[Uuid], model: impl serde::Serialize) {
        let serialized_model = serde_json::to_string(&model).unwrap();

        for user in users {
            if let Some(value) = self.map.get(user) {
                value.send(serialized_model.to_owned()).unwrap();
            }
        }
    }
}

struct GeplauderWebSocket {
    user_id: Uuid,
    receiver: Option<Receiver<String>>,
}

impl Actor for GeplauderWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let (tx, rx) = channel::<String>();
        self.receiver = Some(rx);

        CHANNELS.lock().unwrap().add_user(self.user_id, tx);

        ctx.run_interval(Duration::from_millis(50), |act, ctx| {
            while let Ok(value) = act.receiver.as_ref().unwrap().try_recv() {
                ctx.text(value);
            }
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        CHANNELS.lock().unwrap().remove_user(self.user_id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GeplauderWebSocket {
    fn handle(&mut self, _item: Result<ws::Message, ws::ProtocolError>, _ctx: &mut Self::Context) {
        // match item {
        //     Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
        //     Ok(ws::Message::Text(text)) => ctx.text(text),
        //     Ok(ws::Message::Binary(binary)) => ctx.binary(binary),
        //     _ => (),
        // }
    }
}

pub async fn websocket(
    request: HttpRequest,
    stream: web::Payload,
    auth: AuthorizationService,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(
        GeplauderWebSocket {
            user_id: auth.claims.id,
            receiver: None,
        },
        &request,
        stream,
    )
}
