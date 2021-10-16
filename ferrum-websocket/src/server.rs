use std::collections::HashMap;

use async_trait::async_trait;
use ferrum_db::{
    channels::queries::{get_channels_for_server, get_channels_for_user},
    servers::queries::{get_server_with_id, get_servers_for_user},
    users::queries::{get_user_with_id, get_users_on_server},
};
use ferrum_shared::{channels::ChannelResponse, servers::ServerResponse, users::UserResponse};
use meio::{ActionHandler, Actor, Address, Context, StartedBy, System};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{messages::BrokerEvent, WebSocketSession};

use super::messages::{
    IdentifyUser, /*, NewChannel, NewServer, NewUser*/
    SendMessageToChannel, SerializedWebSocketMessage, WebSocketClose, WebSocketMessage,
};

///
/// Manages all [`crate::WebSocketSession`] and updates them with appropriate events.
///
pub struct WebSocketServer {
    pub db_pool: PgPool,
    pub users: HashMap<Uuid, Address<WebSocketSession>>,
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
    pub async fn send_message_to_channel(&self, channel_id: Uuid, message: WebSocketMessage) {
        let client_message =
            SerializedWebSocketMessage::Data(serde_json::to_string(&message).unwrap(), channel_id);

        for mut recipient in self.users.values().cloned() {
            recipient.act(client_message.clone()).await.expect("");
        }
    }

    fn identify_user(&mut self, user_id: Uuid, recipient: Address<WebSocketSession>) {
        self.users.insert(user_id, recipient);
    }

    fn close_user(&mut self, user_id: Uuid) {
        self.users.remove(&user_id);
    }

    async fn new_channel(&mut self, channel: ChannelResponse) {
        // Get all users that should be notified about the new channel and send it to them
        let affected_users = get_users_on_server(channel.server_id, &self.db_pool)
            .await
            .unwrap();

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(SerializedWebSocketMessage::AddChannel(channel.clone()))
                    .await
                    .unwrap();
            }
        }
    }

    async fn new_server(&mut self, user_id: Uuid, server_id: Uuid) {
        // Get the server and transform it into a response
        let server: ServerResponse = get_server_with_id(server_id, &self.db_pool)
            .await
            .unwrap()
            .into();

        // Get all channels and users of this server and transform them into proper responses
        let channels: Vec<ChannelResponse> = get_channels_for_server(server_id, &self.db_pool)
            .await
            .unwrap()
            .iter()
            .map(|x| x.clone().into())
            .collect();

        let users_on_server: Vec<UserResponse> = get_users_on_server(server_id, &self.db_pool)
            .await
            .unwrap()
            .iter()
            .map(|x| x.clone().into())
            .collect();

        // Send them all to the new servers' owner
        if let Some(recipient) = self.users.get_mut(&user_id) {
            recipient
                .act(SerializedWebSocketMessage::AddServer(
                    server.clone(),
                    channels,
                    users_on_server,
                ))
                .await
                .unwrap();
        }
    }

    async fn new_user(&mut self, user_id: Uuid, server_id: Uuid) {
        // Get the new user and transform them into a response
        let new_user: UserResponse = get_user_with_id(user_id, &self.db_pool)
            .await
            .unwrap()
            .into();

        // Send the new user to all websocket sessions, letting them reject it if necessary
        for (user_id, mut recipient) in self.users.clone() {
            if user_id == user_id {
                continue;
            }

            if let Err(error) = recipient
                .act(SerializedWebSocketMessage::AddUser(
                    server_id,
                    new_user.clone(),
                ))
                .await
            {
                println!("Error in NewUser websocket message handler: {:?}", error);
            }
        }
    }

    async fn user_left(&mut self, user_id: Uuid, server_id: Uuid) {
        if let Some(recipient) = self.users.get_mut(&user_id) {
            recipient
                .act(SerializedWebSocketMessage::DeleteServer(server_id))
                .await
                .unwrap();
        }

        // Get all users that are on the server and notify them about the leaving user
        let affected_users = get_users_on_server(server_id, &self.db_pool).await.unwrap();

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(SerializedWebSocketMessage::DeleteUser(user_id, server_id))
                    .await
                    .unwrap();
            }
        }
    }

    async fn delete_server(&mut self, server_id: Uuid) {
        // Send the deleted server to all websocket sessions, letting them reject it if necessary
        for mut recipient in self.users.values().cloned() {
            recipient
                .act(SerializedWebSocketMessage::DeleteServer(server_id))
                .await
                .unwrap();
        }
    }

    async fn update_server(&mut self, server_id: Uuid) {
        // Get updated server response
        let server: ServerResponse = get_server_with_id(server_id, &self.db_pool)
            .await
            .unwrap()
            .into();

        // Get all users that are on the server and notify them about the updated server
        let affected_users = get_users_on_server(server_id, &self.db_pool).await.unwrap();

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(SerializedWebSocketMessage::UpdateServer(server.clone()))
                    .await
                    .unwrap();
            }
        }
    }
}

#[async_trait]
impl StartedBy<System> for WebSocketServer {
    async fn handle(&mut self, _ctx: &mut Context<Self>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

impl Actor for WebSocketServer {
    type GroupBy = ();
}

#[async_trait]
impl ActionHandler<IdentifyUser> for WebSocketServer {
    async fn handle(
        &mut self,
        mut msg: IdentifyUser,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        // Store the user
        self.identify_user(msg.user_id, msg.addr.clone());

        let user_id = msg.user_id;

        // Get all channels and servers for this user and inform their websocket session about them
        let channels = get_channels_for_user(user_id, &self.db_pool).await.unwrap();
        let servers = get_servers_for_user(user_id, &self.db_pool).await.unwrap();

        msg.addr
            .act(SerializedWebSocketMessage::Ready(
                servers.iter().map(|x| x.id).collect(),
                channels.iter().map(|x| x.id).collect(),
            ))
            .await
            .expect("");

        Ok(())
    }
}

#[async_trait]
impl ActionHandler<WebSocketClose> for WebSocketServer {
    async fn handle(
        &mut self,
        msg: WebSocketClose,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        self.close_user(msg.user_id);

        Ok(())
    }
}

#[async_trait]
impl ActionHandler<SendMessageToChannel> for WebSocketServer {
    async fn handle(
        &mut self,
        msg: SendMessageToChannel,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        self.send_message_to_channel(msg.channel_id, msg.message)
            .await;

        Ok(())
    }
}

// #[async_trait]
// impl ActionHandler<NewChannel> for WebSocketServer {
//     async fn handle(
//         &mut self,
//         msg: NewChannel,
//         _ctx: &mut Context<Self>,
//     ) -> Result<(), anyhow::Error> {
//         // Get all users that should be notified about the new channel and send it to them
//         let affected_users = get_users_on_server(msg.channel.server_id, &self.db_pool)
//             .await
//             .unwrap();

//         for user in &affected_users {
//             if let Some(recipient) = self.users.get_mut(&user.id) {
//                 recipient
//                     .act(SerializedWebSocketMessage::AddChannel(msg.channel.clone()))
//                     .await
//                     .unwrap();
//             }
//         }

//         Ok(())
//     }
// }

// #[async_trait]
// impl ActionHandler<NewServer> for WebSocketServer {
//     async fn handle(
//         &mut self,
//         msg: NewServer,
//         _ctx: &mut Context<Self>,
//     ) -> Result<(), anyhow::Error> {
//         // Get the server and transform it into a response
//         let server: ServerResponse = get_server_with_id(msg.server_id, &self.db_pool)
//             .await
//             .unwrap()
//             .into();

//         // Get all channels and users of this server and transform them into proper responses
//         let channels: Vec<ChannelResponse> = get_channels_for_server(msg.server_id, &self.db_pool)
//             .await
//             .unwrap()
//             .iter()
//             .map(|x| x.clone().into())
//             .collect();

//         let users_on_server: Vec<UserResponse> = get_users_on_server(msg.server_id, &self.db_pool)
//             .await
//             .unwrap()
//             .iter()
//             .map(|x| x.clone().into())
//             .collect();

//         // Send them all to the new servers' owner
//         if let Some(recipient) = self.users.get_mut(&msg.user_id) {
//             recipient
//                 .act(SerializedWebSocketMessage::AddServer(
//                     server.clone(),
//                     channels,
//                     users_on_server,
//                 ))
//                 .await
//                 .unwrap();
//         }

//         Ok(())
//     }
// }

// #[async_trait]
// impl ActionHandler<NewUser> for WebSocketServer {
//     async fn handle(
//         &mut self,
//         msg: NewUser,
//         _ctx: &mut Context<Self>,
//     ) -> Result<(), anyhow::Error> {
//         // Get the new user and transform them into a response
//         let new_user: UserResponse = get_user_with_id(msg.user_id, &self.db_pool)
//             .await
//             .unwrap()
//             .into();

//         // Send the new user to all websocket sessions, letting them reject it if necessary
//         for (user_id, mut recipient) in self.users.clone() {
//             if user_id == msg.user_id {
//                 continue;
//             }

//             if let Err(error) = recipient
//                 .act(SerializedWebSocketMessage::AddUser(
//                     msg.server_id,
//                     new_user.clone(),
//                 ))
//                 .await
//             {
//                 println!("Error in NewUser websocket message handler: {:?}", error);
//             }
//         }

//         Ok(())
//     }
// }

// #[async_trait]
// impl ActionHandler<UserLeft> for WebSocketServer {
//     async fn handle(
//         &mut self,
//         msg: UserLeft,
//         _ctx: &mut Context<Self>,
//     ) -> Result<(), anyhow::Error> {
//         if let Some(recipient) = self.users.get_mut(&msg.user_id) {
//             recipient
//                 .act(SerializedWebSocketMessage::DeleteServer(msg.server_id))
//                 .await
//                 .unwrap();
//         }

//         // Get all users that are on the server and notify them about the leaving user
//         let affected_users = get_users_on_server(msg.server_id, &self.db_pool)
//             .await
//             .unwrap();

//         for user in &affected_users {
//             if let Some(recipient) = self.users.get_mut(&user.id) {
//                 recipient
//                     .act(SerializedWebSocketMessage::DeleteUser(
//                         msg.user_id,
//                         msg.server_id,
//                     ))
//                     .await
//                     .unwrap();
//             }
//         }

//         Ok(())
//     }
// }

// #[async_trait]
// impl ActionHandler<DeleteServer> for WebSocketServer {
//     async fn handle(
//         &mut self,
//         msg: DeleteServer,
//         _ctx: &mut Context<Self>,
//     ) -> Result<(), anyhow::Error> {
//         // Send the deleted server to all websocket sessions, letting them reject it if necessary
//         for mut recipient in self.users.values().cloned() {
//             recipient
//                 .act(SerializedWebSocketMessage::DeleteServer(msg.server_id))
//                 .await
//                 .unwrap();
//         }

//         Ok(())
//     }
// }

// #[async_trait]
// impl ActionHandler<UpdateServer> for WebSocketServer {
//     async fn handle(
//         &mut self,
//         msg: UpdateServer,
//         _ctx: &mut Context<Self>,
//     ) -> Result<(), anyhow::Error> {
//         // Get updated server response
//         let server: ServerResponse = get_server_with_id(msg.server_id, &self.db_pool)
//             .await
//             .unwrap()
//             .into();

//         // Get all users that are on the server and notify them about the updated server
//         let affected_users = get_users_on_server(msg.server_id, &self.db_pool)
//             .await
//             .unwrap();

//         for user in &affected_users {
//             if let Some(recipient) = self.users.get_mut(&user.id) {
//                 recipient
//                     .act(SerializedWebSocketMessage::UpdateServer(server.clone()))
//                     .await
//                     .unwrap();
//             }
//         }

//         Ok(())
//     }
// }

#[async_trait]
impl ActionHandler<BrokerEvent> for WebSocketServer {
    async fn handle(
        &mut self,
        msg: BrokerEvent,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        match msg {
            BrokerEvent::NewChannel { channel } => self.new_channel(channel).await,
            BrokerEvent::NewServer { user_id, server_id } => {
                self.new_server(user_id, server_id).await
            }
            BrokerEvent::NewUser { user_id, server_id } => self.new_user(user_id, server_id).await,
            BrokerEvent::UserLeft { user_id, server_id } => {
                self.user_left(user_id, server_id).await
            }
            BrokerEvent::DeleteServer { server_id } => self.delete_server(server_id).await,
            BrokerEvent::UpdateServer { server_id } => self.update_server(server_id).await,
        }

        Ok(())
    }
}
