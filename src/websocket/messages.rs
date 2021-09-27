use actix::Recipient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::messages::MessageResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct BootstrapPayload {
    pub channels: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewMessagePayload {
    pub message: MessageResponse,
}

#[derive(Debug, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
#[serde(tag = "type", content = "payload")]
pub enum WebSocketMessage {
    Empty,
    Bootstrap(BootstrapPayload),
    NewMessage(NewMessagePayload),
}

#[derive(Debug, Clone, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct SerializedWebSocketMessage(pub String);

#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct WebSocketConnect {
    pub user_id: Uuid,
    pub recipient: Recipient<SerializedWebSocketMessage>,
}

impl WebSocketConnect {
    pub fn new(user_id: Uuid, recipient: Recipient<SerializedWebSocketMessage>) -> Self {
        Self { user_id, recipient }
    }
}

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

#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct RegisterChannelsForUser {
    pub user_id: Uuid,
    pub addr: Recipient<SerializedWebSocketMessage>,
    pub channels: Vec<Uuid>,
}

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
