use meio::Action;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrokerEvent {
    NewChannel { channel_id: Uuid },
    NewServer { user_id: Uuid, server_id: Uuid },
    UserJoined { user_id: Uuid, server_id: Uuid },
    UserLeft { user_id: Uuid, server_id: Uuid },
    DeleteServer { server_id: Uuid },
    DeleteChannel { channel_id: Uuid },
    UpdateServer { server_id: Uuid },
    NewMessage { channel_id: Uuid, message_id: Uuid },
}

impl Action for BrokerEvent {}
