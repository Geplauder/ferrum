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
        if let Some(users) = self.channel_sessions.get(&channel_id) {
            let client_message =
                SerializedWebSocketMessage(serde_json::to_string(&message).unwrap());

            for user in users {
                if let Some(recipient) = self.sessions.get(user) {
                    recipient.do_send(client_message.clone()).expect("");
                }
            }
        }
    }

    fn connect_user(&mut self, user_id: Uuid, recipient: Recipient<SerializedWebSocketMessage>) {
        self.sessions.insert(user_id, recipient);
    }

    fn close_user(&mut self, user_id: Uuid) {
        self.sessions.remove(&user_id);
    }

    fn register_channels_for_user(&mut self, user_id: Uuid, channels: Vec<Uuid>) {
        for entry in self.channel_sessions.values_mut() {
            let mut found_index = None;

            for (index, existing_user_id) in entry.iter().enumerate() {
                if *existing_user_id == user_id {
                    found_index = Some(index);
                }
            }

            if let Some(index) = found_index {
                entry.remove(index);
            }
        }

        for channel in &channels {
            match self.channel_sessions.get_mut(channel) {
                Some(value) => {
                    value.push(user_id);
                }
                None => {
                    self.channel_sessions.insert(*channel, vec![user_id]);
                }
            };
        }
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<WebSocketConnect> for Server {
    type Result = ();

    fn handle(&mut self, msg: WebSocketConnect, _ctx: &mut Self::Context) -> Self::Result {
        self.connect_user(msg.user_id, msg.recipient);
    }
}

impl Handler<WebSocketClose> for Server {
    type Result = ();

    fn handle(&mut self, msg: WebSocketClose, _ctx: &mut Self::Context) -> Self::Result {
        self.close_user(msg.user_id);
    }
}

impl Handler<RegisterChannelsForUser> for Server {
    type Result = ();

    fn handle(&mut self, msg: RegisterChannelsForUser, _ctx: &mut Self::Context) -> Self::Result {
        self.register_channels_for_user(msg.user_id, msg.channels);
    }
}

impl Handler<SendMessageToChannel> for Server {
    type Result = ();

    fn handle(&mut self, msg: SendMessageToChannel, _ctx: &mut Self::Context) -> Self::Result {
        self.send_message_to_channel(msg.channel_id, msg.message);
    }
}

#[cfg(test)]
mod tests {
    use actix::actors::mocker::Mocker;

    use super::*;
    use crate::routes::websocket::WebSocketSession;

    type WebSocketSessionMock = Mocker<WebSocketSession>;

    #[actix_rt::test]
    async fn register_channels_is_successful_for_connected_users() {
        let mut server = Server::default();

        let user_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();

        let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
        let actor = mocker.start();
        server.connect_user(user_id, actor.recipient());

        server.register_channels_for_user(user_id, vec![channel_id]);

        assert_eq!(1, server.sessions.len());
        assert_eq!(1, server.channel_sessions.len());
    }

    #[actix_rt::test]
    async fn register_channels_is_successful_for_multiple_connected_users() {
        let mut server = Server::default();

        let first_user_id = Uuid::new_v4();
        let second_user_id = Uuid::new_v4();

        let channel_id = Uuid::new_v4();

        let first_mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
        let first_actor = first_mocker.start();
        server.connect_user(first_user_id, first_actor.recipient());

        let second_mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
        let second_actor = second_mocker.start();
        server.connect_user(second_user_id, second_actor.recipient());

        server.register_channels_for_user(first_user_id, vec![channel_id]);
        assert_eq!(1, server.channel_sessions.len());
        {
            let users = server.channel_sessions.get(&channel_id).unwrap();
            assert_eq!(1, users.len());
        }

        server.register_channels_for_user(second_user_id, vec![channel_id]);
        assert_eq!(1, server.channel_sessions.len());
        {
            let users = server.channel_sessions.get(&channel_id).unwrap();
            assert_eq!(2, users.len());
        }
    }

    #[actix_rt::test]
    async fn register_channels_cleans_up_channel_sessions_on_additional_calls() {
        let mut server = Server::default();

        let user_id = Uuid::new_v4();
        let first_channel_id = Uuid::new_v4();
        let second_channel_id = Uuid::new_v4();

        let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
        let actor = mocker.start();
        server.connect_user(user_id, actor.recipient());

        server.register_channels_for_user(user_id, vec![first_channel_id]);

        {
            let users = server.channel_sessions.get(&first_channel_id).unwrap();
            assert_eq!(user_id, users[0]);
        }

        server.register_channels_for_user(user_id, vec![second_channel_id]);

        {
            let users = server.channel_sessions.get(&first_channel_id).unwrap();
            assert!(users.is_empty());

            let users = server.channel_sessions.get(&second_channel_id).unwrap();
            assert_eq!(user_id, users[0]);
        }
    }

    #[actix_rt::test]
    async fn send_message_only_sends_to_users_in_channel_session() {
        let mut server = Server::default();

        let user_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();

        let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| {
            assert!(
                false,
                "User received message despite not being in channel session"
            );

            Box::new(())
        }));

        let actor = mocker.start();
        server.connect_user(user_id, actor.recipient());

        server.send_message_to_channel(channel_id, WebSocketMessage::Empty);
    }

    #[actix_rt::test]
    async fn send_message_only_sends_to_users_that_have_a_session() {
        let mut server = Server::default();

        let user_id = Uuid::new_v4();
        let channel_id = Uuid::new_v4();

        let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| {
            assert!(false, "User received message despite not having a session");

            Box::new(())
        }));

        let _ = mocker.start();
        server.register_channels_for_user(user_id, vec![channel_id]);

        println!("{:#?}", server.channel_sessions);

        server.send_message_to_channel(channel_id, WebSocketMessage::Empty);
    }

    #[actix_rt::test]
    async fn close_user_removes_user_from_sessions() {
        let mut server = Server::default();

        let user_id = Uuid::new_v4();

        let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
        let actor = mocker.start();
        server.connect_user(user_id, actor.recipient());

        assert_eq!(1, server.sessions.len());

        server.close_user(user_id);

        assert_eq!(0, server.sessions.len());
    }
}
