use ferrum_shared::{
    channels::ChannelResponse, messages::MessageResponse, servers::ServerResponse,
    users::UserResponse,
};
use meio::{Action, Address};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WebSocketSession;

///
/// These message are received from and sent to the actual websocket client.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SerializedWebSocketMessage {
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
    /// Sent to the client to inform them that they have no longer access to a channel.
    DeleteChannel { channel_id: Uuid },
    /// Sent to the client to inform them about the updated server.
    UpdateServer { server: ServerResponse },
}

impl Action for SerializedWebSocketMessage {}

///
/// These messages are sent from the [`crate::server::WebSocketServer`] to the [`crate::WebSocketSession`].
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketSessionMessage {
    /// Tells the [`crate::WebSocketSession`] to which servers it belongs.
    Ready(Vec<Uuid>),
    /// Tells the [`crate::WebSocketSession`] to notify the client about a new message.
    AddMessage(MessageResponse),
    /// Adds a channel to the [`crate::WebSocketSession`] and tells it to notify the client about it.
    AddChannel(ChannelResponse),
    /// Adds a server to the [`crate::WebSocketSession`] and tells it to notify the client about it.
    AddServer(ServerResponse, Vec<ChannelResponse>, Vec<UserResponse>),
    /// Tells the [`crate::WebSocketSession`] to notify the client about a new user.
    AddUser(Uuid, UserResponse),
    /// Removes a server from the [`crate::WebSocketSession`] and tells it to notify the client about it.
    DeleteServer(Uuid),
    /// Removes a channel from the [`crate::WebSocketSession`] and tells it to notify the client about it.
    DeleteChannel(Uuid),
    /// Removes a channel from the [`crate::WebSocketSession`] and tells it to notify the client about it.
    DeleteUser(Uuid, Uuid),
    /// Tells the [`crate::WebSocketSession`] to notify the client about a updated server.
    UpdateServer(ServerResponse),
}

impl Action for WebSocketSessionMessage {}

///
/// Message to notify the [`crate::server::WebSocketServer`] about closing [`crate::WebSocketSession`].
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
/// Message to notify the [`crate::server::WebSocketServer`] about identifying clients.
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
