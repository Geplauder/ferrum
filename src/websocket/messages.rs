use actix::Recipient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::messages::MessageResponse;

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentifyPayload {
    pub bearer: String,
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
    Ready,
    Identify(IdentifyPayload),
    NewMessage(NewMessagePayload),
}

// #[derive(Debug, Clone, Serialize, Deserialize, actix::prelude::Message)]
// #[rtype(result = "()")]
// pub struct SerializedWebSocketMessage(pub String, pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
pub enum SerializedWebSocketMessage {
    Ready(Vec<Uuid>),
    Data(String, Uuid),
}

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
pub struct IdentifyUser {
    pub user_id: Uuid,
    pub addr: Recipient<SerializedWebSocketMessage>,
}

#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct ReadyUser {
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
