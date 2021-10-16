use ferrum_shared::{
    channels::ChannelResponse, messages::MessageResponse, servers::ServerResponse,
    users::UserResponse,
};
use meio::{Action, Address};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WebSocketSession;

///
/// These messages are sent and received via the websocket server.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Sent to the client to inform them about the updated server.
    UpdateServer { server: ServerResponse },
}

impl Action for WebSocketMessage {}

///
/// These messages are sent from the websocket [`crate::server::WebSocketServer`] to the [`crate::WebSocketSession`]
///
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Tells the [`crate::WebSocketSession`] to notify the client about a updated server.
    UpdateServer(ServerResponse),
    /// Tells the [`crate::WebSocketSession`] to send raw data to the client.
    Data(String, Uuid),
}

impl Action for SerializedWebSocketMessage {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrokerEvent {
    // TODO: Just use UUID
    NewChannel {
        channel: ChannelResponse,
    },
    NewServer {
        user_id: Uuid,
        server_id: Uuid,
    },
    NewUser {
        user_id: Uuid,
        server_id: Uuid,
    },
    UserLeft {
        user_id: Uuid,
        server_id: Uuid,
    },
    DeleteServer {
        server_id: Uuid,
    },
    UpdateServer {
        server_id: Uuid,
    },
    // TODO: Rework this
    SendMessageToChannel {
        channel_id: Uuid,
        message: WebSocketMessage,
    },
}

impl Action for BrokerEvent {}

///
/// Message to notify the websocket server about closing client sessions.
///
#[derive(Debug)]
pub struct WebSocketClose {
    pub user_id: Uuid,
}

impl Action for WebSocketClose {}

impl WebSocketClose {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }
}

///
/// Message to notify the websocket server about identifying clients.
///
#[derive(Debug)]
pub struct IdentifyUser {
    pub user_id: Uuid,
    pub addr: Address<WebSocketSession>,
}

impl Action for IdentifyUser {}

#[derive(Debug)]
pub struct ReadyUser {
    pub channels: Vec<Uuid>,
}
