use actix::Recipient;
use ferrum_shared::{
    channels::ChannelResponse, messages::MessageResponse, servers::ServerResponse,
    users::UserResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

///
/// These messages are sent and received via the websocket server.
///
#[derive(Debug, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
#[serde(tag = "type", content = "payload")]
pub enum WebSocketMessage {
    /// Empty message.
    Empty,
    /// Sent periodically from the client, the server has to respond with a [`WebSocketMessage::Pong`].
    Ping,
    /// Sent as a response to a [`WebSocketMessage::Ping`] message.
    Pong,
    /// Sent to the client to indicate that the handshake was successful.
    Ready,
    /// Sent from the client for authentication purposes.
    Identify { bearer: String },
    /// Sent to the client to inform about a new message.
    NewMessage { message: MessageResponse },
    /// Sent to the client to inform about a new channel.
    NewChannel { channel: ChannelResponse },
    /// Sent to the client to inform about a new server.
    NewServer {
        server: ServerResponse,
        channels: Vec<ChannelResponse>,
        users: Vec<UserResponse>,
    },
    /// Sent to the client to inform about a new user.
    NewUser { server_id: Uuid, user: UserResponse },
    /// Sent to the client to inform them that a user is no longer part of a server.
    DeleteUser { server_id: Uuid, user_id: Uuid },
    /// Sent to the client to inform them that they have no longer access to a server.
    DeleteServer { server_id: Uuid },
}

///
/// These messages are sent from the websocket [`crate::server::WebSocketServer`] to the [`crate::WebSocketSession`]
///
#[derive(Debug, Clone, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
pub enum SerializedWebSocketMessage {
    /// Tells the [`crate::WebSocketSession`] to which servers and channels it belongs.
    Ready(Vec<Uuid>, Vec<Uuid>),
    /// Adds a channel to the [`crate::WebSocketSession`] and tells it to notify the client about it.
    AddChannel(ChannelResponse),
    /// Adds a server to the [`crate::WebSocketSession`] and tells it to notify the client about it.
    AddServer(ServerResponse, Vec<ChannelResponse>, Vec<UserResponse>),
    /// Tells the [`crate::WebSocketSession`] to notify the client about a new user.
    AddUser(Uuid, UserResponse),
    /// Removes a server from the [`crate::WebSocketSession`] and tells it to notify the client about it.
    DeleteServer(Uuid),
    /// Removes a channel from the [`crate::WebSocketSession`] and tells it to notify the client about it.
    DeleteUser(Uuid, Uuid),
    /// Tells the [`crate::WebSocketSession`] to send raw data to the client.
    Data(String, Uuid),
}

///
/// Message to notify the websocket server about closing client sessions.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct WebSocketClose {
    pub user_id: Uuid,
}

impl WebSocketClose {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }
}

///
/// Message to notify the websocket server about identifying clients.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct IdentifyUser {
    pub user_id: Uuid,
    pub addr: Recipient<SerializedWebSocketMessage>,
}

#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct ReadyUser {
    pub channels: Vec<Uuid>,
}

///
/// Message to notify the websocket server about a new message in a channel.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct SendMessageToChannel {
    pub channel_id: Uuid,
    pub message: WebSocketMessage,
}

impl SendMessageToChannel {
    pub fn new(channel_id: Uuid, message: WebSocketMessage) -> Self {
        Self {
            channel_id,
            message,
        }
    }
}

///
/// Message to notify the websocket server about a new channel.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct NewChannel {
    pub channel: ChannelResponse,
}

impl NewChannel {
    pub fn new(channel: ChannelResponse) -> Self {
        Self { channel }
    }
}

///
/// Message to notify the websocket server about a new server.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct NewServer {
    pub user_id: Uuid,
    pub server_id: Uuid,
}

impl NewServer {
    pub fn new(user_id: Uuid, server_id: Uuid) -> Self {
        Self { user_id, server_id }
    }
}

///
/// Message to notify the websocket server about a new user.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct NewUser {
    pub user_id: Uuid,
    pub server_id: Uuid,
}

impl NewUser {
    pub fn new(user_id: Uuid, server_id: Uuid) -> Self {
        Self { user_id, server_id }
    }
}

///
/// Message to notify the websocket server about a leaving user.
///
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct UserLeft {
    pub user_id: Uuid,
    pub server_id: Uuid,
}

impl UserLeft {
    pub fn new(user_id: Uuid, server_id: Uuid) -> Self {
        Self { user_id, server_id }
    }
}

///
/// Message to notify the websocket server about a deleted server.
#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct DeleteServer {
    pub server_id: Uuid,
}

impl DeleteServer {
    pub fn new(server_id: Uuid) -> Self {
        Self { server_id }
    }
}
