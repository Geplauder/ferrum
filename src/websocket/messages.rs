use actix::Recipient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    channels::Channel, messages::MessageResponse, servers::Server, users::UserResponse,
};

#[derive(Debug, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
#[serde(tag = "type", content = "payload")]
pub enum WebSocketMessage {
    Empty,
    Ping,
    Pong,
    Ready,
    Identify {
        bearer: String,
    },
    NewMessage {
        message: MessageResponse,
    },
    NewChannel {
        channel: Channel,
    },
    NewServer {
        server: Server,
        channels: Vec<Channel>,
    },
    NewUser {
        server_id: Uuid,
        user: UserResponse,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
pub enum SerializedWebSocketMessage {
    Ready(Vec<Uuid>),
    AddChannel(Channel),
    AddServer(Server, Vec<Channel>), // TODO: Add user(s)
    AddUser(Uuid, UserResponse),
    Data(String, Uuid),
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

#[derive(Debug, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct NewChannel {
    pub channel: Channel,
}

impl NewChannel {
    pub fn new(channel: Channel) -> Self {
        Self { channel }
    }
}

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
