use std::collections::HashMap;

use actix::{Actor, Context, ContextFutureSpawner, Handler, Recipient, WrapFuture};
use sqlx::PgPool;
use uuid::Uuid;

use super::messages::{
    IdentifyUser, SendMessageToChannel, SerializedWebSocketMessage, WebSocketClose,
    WebSocketMessage,
};

pub struct Server {
    db_pool: PgPool,
    users: HashMap<Uuid, Recipient<SerializedWebSocketMessage>>,
}

impl Server {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            users: HashMap::new(),
        }
    }

    pub fn send_message_to_channel(&self, channel_id: Uuid, message: WebSocketMessage) {
        let client_message =
            SerializedWebSocketMessage::Data(serde_json::to_string(&message).unwrap(), channel_id);

        for (_user, recipient) in &self.users {
            recipient.do_send(client_message.clone()).expect("");
        }
    }

    fn identify_user(&mut self, user_id: Uuid, recipient: Recipient<SerializedWebSocketMessage>) {
        self.users.insert(user_id, recipient);
    }

    fn close_user(&mut self, user_id: Uuid) {
        self.users.remove(&user_id);
    }
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<IdentifyUser> for Server {
    type Result = ();

    fn handle(&mut self, msg: IdentifyUser, ctx: &mut Self::Context) -> Self::Result {
        self.identify_user(msg.user_id, msg.addr.clone());

        let addr = msg.addr.clone();
        let db_pool = self.db_pool.clone();

        async move {
            let channels = sqlx::query!(
                r#"
                WITH server_query AS (SELECT servers.id as server_id
                    FROM users_servers
                    INNER JOIN servers ON servers.id = users_servers.server_id
                )
                SELECT channels.id
                FROM channels
                WHERE channels.server_id IN (SELECT server_id FROM server_query)
                "#
            )
            .fetch_all(&db_pool)
            .await
            .unwrap();

            addr.do_send(SerializedWebSocketMessage::Ready(
                channels.iter().map(|x| x.id).collect(),
            ))
            .expect("");
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<WebSocketClose> for Server {
    type Result = ();

    fn handle(&mut self, msg: WebSocketClose, _ctx: &mut Self::Context) -> Self::Result {
        self.close_user(msg.user_id);
    }
}

impl Handler<SendMessageToChannel> for Server {
    type Result = ();

    fn handle(&mut self, msg: SendMessageToChannel, _ctx: &mut Self::Context) -> Self::Result {
        self.send_message_to_channel(msg.channel_id, msg.message);
    }
}

// #[cfg(test)]
// mod tests {
//     use actix::actors::mocker::Mocker;

//     use super::*;
//     use crate::routes::websocket::WebSocketSession;

//     type WebSocketSessionMock = Mocker<WebSocketSession>;

// #[actix_rt::test]
// async fn register_channels_is_successful_for_connected_users() {
//     let mut server = Server::default();

//     let user_id = Uuid::new_v4();
//     let channel_id = Uuid::new_v4();

//     let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
//     let actor = mocker.start();
//     server.connect_user(user_id, actor.recipient());

//     server.register_channels_for_user(user_id, vec![channel_id]);

//     assert_eq!(1, server.sessions.len());
//     assert_eq!(1, server.channel_sessions.len());
// }

// #[actix_rt::test]
// async fn register_channels_is_successful_for_multiple_connected_users() {
//     let mut server = Server::default();

//     let first_user_id = Uuid::new_v4();
//     let second_user_id = Uuid::new_v4();

//     let channel_id = Uuid::new_v4();

//     let first_mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
//     let first_actor = first_mocker.start();
//     server.connect_user(first_user_id, first_actor.recipient());

//     let second_mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
//     let second_actor = second_mocker.start();
//     server.connect_user(second_user_id, second_actor.recipient());

//     server.register_channels_for_user(first_user_id, vec![channel_id]);
//     assert_eq!(1, server.channel_sessions.len());
//     {
//         let users = server.channel_sessions.get(&channel_id).unwrap();
//         assert_eq!(1, users.len());
//     }

//     server.register_channels_for_user(second_user_id, vec![channel_id]);
//     assert_eq!(1, server.channel_sessions.len());
//     {
//         let users = server.channel_sessions.get(&channel_id).unwrap();
//         assert_eq!(2, users.len());
//     }
// }

// #[actix_rt::test]
// async fn register_channels_cleans_up_channel_sessions_on_additional_calls() {
//     let mut server = Server::default();

//     let user_id = Uuid::new_v4();
//     let first_channel_id = Uuid::new_v4();
//     let second_channel_id = Uuid::new_v4();

//     let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
//     let actor = mocker.start();
//     server.connect_user(user_id, actor.recipient());

//     server.register_channels_for_user(user_id, vec![first_channel_id]);

//     {
//         let users = server.channel_sessions.get(&first_channel_id).unwrap();
//         assert_eq!(user_id, users[0]);
//     }

//     server.register_channels_for_user(user_id, vec![second_channel_id]);

//     {
//         let users = server.channel_sessions.get(&first_channel_id).unwrap();
//         assert!(users.is_empty());

//         let users = server.channel_sessions.get(&second_channel_id).unwrap();
//         assert_eq!(user_id, users[0]);
//     }
// }

// #[actix_rt::test]
// async fn send_message_only_sends_to_users_in_channel_session() {
//     let mut server = Server::default();

//     let user_id = Uuid::new_v4();
//     let channel_id = Uuid::new_v4();

//     let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| {
//         assert!(
//             false,
//             "User received message despite not being in channel session"
//         );

//         Box::new(())
//     }));

//     let actor = mocker.start();
//     server.connect_user(user_id, actor.recipient());

//     server.send_message_to_channel(channel_id, WebSocketMessage::Empty);
// }

// #[actix_rt::test]
// async fn send_message_only_sends_to_users_that_have_a_session() {
//     let mut server = Server::default();

//     let user_id = Uuid::new_v4();
//     let channel_id = Uuid::new_v4();

//     let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| {
//         assert!(false, "User received message despite not having a session");

//         Box::new(())
//     }));

//     let _ = mocker.start();
//     server.register_channels_for_user(user_id, vec![channel_id]);

//     println!("{:#?}", server.channel_sessions);

//     server.send_message_to_channel(channel_id, WebSocketMessage::Empty);
// }

// #[actix_rt::test]
// async fn close_user_removes_user_from_sessions() {
//     let mut server = Server::default();

//     let user_id = Uuid::new_v4();

//     let mocker = WebSocketSessionMock::mock(Box::new(move |_msg, _ctx| Box::new(())));
//     let actor = mocker.start();
//     server.connect_user(user_id, actor.recipient());

//     assert_eq!(1, server.sessions.len());

//     server.close_user(user_id);

//     assert_eq!(0, server.sessions.len());
// }
// }
