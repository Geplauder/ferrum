use std::collections::HashMap;

use actix::{Actor, Context, ContextFutureSpawner, Handler, Recipient, WrapFuture};
use ferrum_db::{
    channels::queries::{get_channels_for_server, get_channels_for_user},
    servers::queries::get_server_with_id,
    users::queries::{get_user_with_id, get_users_on_server},
};
use ferrum_shared::{channels::ChannelResponse, servers::ServerResponse, users::UserResponse};
use sqlx::PgPool;
use uuid::Uuid;

use super::messages::{
    IdentifyUser, NewChannel, NewServer, NewUser, SendMessageToChannel, SerializedWebSocketMessage,
    WebSocketClose, WebSocketMessage,
};

pub struct WebSocketServer {
    db_pool: PgPool,
    users: HashMap<Uuid, Recipient<SerializedWebSocketMessage>>,
}

impl WebSocketServer {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            users: HashMap::new(),
        }
    }

    pub fn send_message_to_channel(&self, channel_id: Uuid, message: WebSocketMessage) {
        let client_message =
            SerializedWebSocketMessage::Data(serde_json::to_string(&message).unwrap(), channel_id);

        for recipient in self.users.values() {
            recipient.do_send(client_message.clone()).expect("");
        }
    }

    fn identify_user(&mut self, user_id: Uuid, recipient: Recipient<SerializedWebSocketMessage>) {
        self.users.insert(user_id, recipient);
    }

    fn close_user(&mut self, user_id: Uuid) {
        self.users.remove(&user_id);
    }
}

impl Actor for WebSocketServer {
    type Context = Context<Self>;
}

impl Handler<IdentifyUser> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: IdentifyUser, ctx: &mut Self::Context) -> Self::Result {
        self.identify_user(msg.user_id, msg.addr.clone());

        let addr = msg.addr;
        let db_pool = self.db_pool.clone();

        let user_id = msg.user_id;

        async move {
            let channels = get_channels_for_user(user_id, &db_pool).await.unwrap();

            addr.do_send(SerializedWebSocketMessage::Ready(
                channels.iter().map(|x| x.id).collect(),
            ))
            .expect("");
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<WebSocketClose> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: WebSocketClose, _ctx: &mut Self::Context) -> Self::Result {
        self.close_user(msg.user_id);
    }
}

impl Handler<SendMessageToChannel> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: SendMessageToChannel, _ctx: &mut Self::Context) -> Self::Result {
        self.send_message_to_channel(msg.channel_id, msg.message);
    }
}

impl Handler<NewChannel> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: NewChannel, ctx: &mut Self::Context) -> Self::Result {
        let db_pool = self.db_pool.clone();
        let users = self.users.clone();

        async move {
            let affected_users = get_users_on_server(msg.channel.server_id, &db_pool)
                .await
                .unwrap();

            for user in &affected_users {
                if let Some(recipient) = users.get(&user.id) {
                    recipient
                        .do_send(SerializedWebSocketMessage::AddChannel(msg.channel.clone()))
                        .unwrap();
                }
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<NewServer> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: NewServer, ctx: &mut Self::Context) -> Self::Result {
        let db_pool = self.db_pool.clone();
        let users = self.users.clone();

        async move {
            let server: ServerResponse = get_server_with_id(msg.server_id, &db_pool)
                .await
                .unwrap()
                .into();

            let channels: Vec<ChannelResponse> = get_channels_for_server(msg.server_id, &db_pool)
                .await
                .unwrap()
                .iter()
                .map(|x| x.clone().into())
                .collect();

            let users_on_server: Vec<UserResponse> = get_users_on_server(msg.server_id, &db_pool)
                .await
                .unwrap()
                .iter()
                .map(|x| x.clone().into())
                .collect();

            if let Some(recipient) = users.get(&msg.user_id) {
                recipient
                    .do_send(SerializedWebSocketMessage::AddServer(
                        server.clone(),
                        channels,
                        users_on_server,
                    ))
                    .unwrap();
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<NewUser> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: NewUser, ctx: &mut Self::Context) -> Self::Result {
        let db_pool = self.db_pool.clone();
        let users = self.users.clone();

        async move {
            let new_user: UserResponse = get_user_with_id(msg.user_id, &db_pool)
                .await
                .unwrap()
                .into();

            let users_on_server = get_users_on_server(msg.server_id, &db_pool).await.unwrap();

            for user in &users_on_server {
                if let Some(recipient) = users.get(&user.id) {
                    if let Err(error) = recipient.do_send(SerializedWebSocketMessage::AddUser(
                        msg.server_id,
                        new_user.clone(),
                    )) {
                        println!("Error in NewUser websocket message handler: {:?}", error);
                    }
                }
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}
