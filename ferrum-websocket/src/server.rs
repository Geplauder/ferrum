use std::collections::HashMap;

use actix::{Actor, Context, ContextFutureSpawner, Handler, Recipient, WrapFuture};
use ferrum_db::{
    channels::queries::{get_channels_for_server, get_channels_for_user},
    servers::queries::{get_server_with_id, get_servers_for_user},
    users::queries::{get_user_with_id, get_users_on_server},
};
use ferrum_shared::{channels::ChannelResponse, servers::ServerResponse, users::UserResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::messages::{DeleteServer, UpdateServer, UserLeft};

use super::messages::{
    IdentifyUser, NewChannel, NewServer, NewUser, SendMessageToChannel, SerializedWebSocketMessage,
    WebSocketClose, WebSocketMessage,
};

///
/// Manages all [`crate::WebSocketSession`] and updates them with appropriate events.
///
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

    ///
    /// Send a websocket message to all [`crate::WebSocketSession`] that have access to a specific channel.
    ///
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
        // Store the user
        self.identify_user(msg.user_id, msg.addr.clone());

        let addr = msg.addr;
        let db_pool = self.db_pool.clone();

        let user_id = msg.user_id;

        async move {
            // Get all channels and servers for this user and inform their websocket session about them
            let channels = get_channels_for_user(user_id, &db_pool).await.unwrap();
            let servers = get_servers_for_user(user_id, &db_pool).await.unwrap();

            addr.do_send(SerializedWebSocketMessage::Ready(
                servers.iter().map(|x| x.id).collect(),
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
            // Get all users that should be notified about the new channel and send it to them
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
            // Get the server and transform it into a response
            let server: ServerResponse = get_server_with_id(msg.server_id, &db_pool)
                .await
                .unwrap()
                .into();

            // Get all channels and users of this server and transform them into proper responses
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

            // Send them all to the new servers' owner
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
            // Get the new user and transform them into a response
            let new_user: UserResponse = get_user_with_id(msg.user_id, &db_pool)
                .await
                .unwrap()
                .into();

            // Send the new user to all websocket sessions, letting them reject it if necessary
            for (user_id, recipient) in &users {
                if *user_id == msg.user_id {
                    continue;
                }

                if let Err(error) = recipient.do_send(SerializedWebSocketMessage::AddUser(
                    msg.server_id,
                    new_user.clone(),
                )) {
                    println!("Error in NewUser websocket message handler: {:?}", error);
                }
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<UserLeft> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: UserLeft, ctx: &mut Self::Context) -> Self::Result {
        if let Some(recipient) = self.users.get(&msg.user_id) {
            recipient
                .do_send(SerializedWebSocketMessage::DeleteServer(msg.server_id))
                .unwrap();
        }

        let db_pool = self.db_pool.clone();
        let users = self.users.clone();

        async move {
            // Get all users that are on the server and notify them about the leaving user
            let affected_users = get_users_on_server(msg.server_id, &db_pool).await.unwrap();

            for user in &affected_users {
                if let Some(recipient) = users.get(&user.id) {
                    recipient
                        .do_send(SerializedWebSocketMessage::DeleteUser(
                            msg.user_id,
                            msg.server_id,
                        ))
                        .unwrap();
                }
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<DeleteServer> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: DeleteServer, _ctx: &mut Self::Context) -> Self::Result {
        // Send the deleted server to all websocket sessions, letting them reject it if necessary
        for recipient in self.users.values() {
            recipient
                .do_send(SerializedWebSocketMessage::DeleteServer(msg.server_id))
                .unwrap();
        }
    }
}

impl Handler<UpdateServer> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: UpdateServer, ctx: &mut Self::Context) -> Self::Result {
        let db_pool = self.db_pool.clone();
        let users = self.users.clone();

        async move {
            // Get updated server response
            let server: ServerResponse = get_server_with_id(msg.server_id, &db_pool)
                .await
                .unwrap()
                .into();

            // Get all users that are on the server and notify them about the updated server
            let affected_users = get_users_on_server(msg.server_id, &db_pool).await.unwrap();

            for user in &affected_users {
                if let Some(recipient) = users.get(&user.id) {
                    recipient
                        .do_send(SerializedWebSocketMessage::UpdateServer(server.clone()))
                        .unwrap();
                }
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}
