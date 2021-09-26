use std::collections::HashMap;

use actix::{Actor, Context, Handler, Recipient};
use uuid::Uuid;

use super::messages::{
    RegisterChannelsForUser, SendMessageToChannel, SerializedWebSocketMessage, WebSocketMessage,
};

#[derive(Default)]
pub struct Server {
    map: HashMap<Uuid, Vec<Recipient<SerializedWebSocketMessage>>>,
}

impl Server {
    pub fn send_message_to_channel(&self, channel_id: Uuid, message: WebSocketMessage) {
        let recipients = match self.map.get(&channel_id) {
            Some(value) => value,
            None => return,
        };

        let client_message = SerializedWebSocketMessage(serde_json::to_string(&message).unwrap());

        for recipient in recipients {
            recipient.do_send(client_message.clone()).expect("");
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<RegisterChannelsForUser> for Server {
    type Result = ();

    fn handle(&mut self, msg: RegisterChannelsForUser, _ctx: &mut Self::Context) -> Self::Result {
        for channel in &msg.channels {
            match self.map.get_mut(channel) {
                Some(value) => {
                    value.push(msg.addr.clone());
                }
                None => {
                    self.map.insert(*channel, vec![msg.addr.clone()]);
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
