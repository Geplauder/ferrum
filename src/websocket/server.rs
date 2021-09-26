use std::collections::HashMap;

use actix::{Actor, Context, Handler, Recipient};
use uuid::Uuid;

use super::messages::{
    RegisterChannelsForUser, SendMessageToChannel, SerializedWebSocketMessage, WebSocketClose,
    WebSocketConnect, WebSocketMessage,
};

#[derive(Default)]
pub struct Server {
    sessions: HashMap<Uuid, Recipient<SerializedWebSocketMessage>>, // Maps user_id to actor recipient
    channel_sessions: HashMap<Uuid, Vec<Uuid>>,                     // Maps channel_id to user_ids
}

impl Server {
    pub fn send_message_to_channel(&self, channel_id: Uuid, message: WebSocketMessage) {
        let users = match self.channel_sessions.get(&channel_id) {
            Some(value) => value,
            None => return,
        };

        let client_message = SerializedWebSocketMessage(serde_json::to_string(&message).unwrap());

        for user in users {
            let recipient = match self.sessions.get(user) {
                Some(value) => value,
                None => continue,
            };

            recipient.do_send(client_message.clone()).expect("");
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<WebSocketConnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: WebSocketConnect, _ctx: &mut Self::Context) -> Self::Result {
        self.sessions.insert(msg.user_id, msg.recipient);
    }
}

impl Handler<WebSocketClose> for Server {
    type Result = ();

    fn handle(&mut self, msg: WebSocketClose, _ctx: &mut Self::Context) -> Self::Result {
        self.sessions.remove(&msg.user_id);
    }
}

impl Handler<RegisterChannelsForUser> for Server {
    type Result = ();

    fn handle(&mut self, msg: RegisterChannelsForUser, _ctx: &mut Self::Context) -> Self::Result {
        for entry in self.channel_sessions.values_mut() {
            let mut found_index = None;

            for (index, existing_user_id) in entry.iter().enumerate() {
                if *existing_user_id == msg.user_id {
                    found_index = Some(index);
                }
            }

            if let Some(index) = found_index {
                entry.remove(index);
            }
        }

        for channel in &msg.channels {
            match self.channel_sessions.get_mut(channel) {
                Some(value) => {
                    value.push(msg.user_id);
                }
                None => {
                    self.channel_sessions.insert(*channel, vec![msg.user_id]);
                }
            };
        }
    }
}

impl Handler<SendMessageToChannel> for Server {
    type Result = ();

    fn handle(&mut self, msg: SendMessageToChannel, _ctx: &mut Self::Context) -> Self::Result {
        self.send_message_to_channel(msg.channel_id, msg.message);
    }
}
