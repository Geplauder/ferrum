use std::collections::HashMap;

use actix::{Actor, Context, ContextFutureSpawner, Handler, Recipient, WrapFuture};
use sqlx::PgPool;
use uuid::Uuid;

use super::messages::{
    IdentifyUser, NewChannel, NewServer, SendMessageToChannel, SerializedWebSocketMessage,
    WebSocketClose, WebSocketMessage,
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

        for recipient in self.users.values() {
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

        let addr = msg.addr;
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

impl Handler<NewChannel> for Server {
    type Result = ();

    fn handle(&mut self, msg: NewChannel, ctx: &mut Self::Context) -> Self::Result {
        let db_pool = self.db_pool.clone();
        let users = self.users.clone();

        async move {
            let affected_users = sqlx::query!(
                r#"
                WITH server_query AS (
                    SELECT servers.id as server_id
                    FROM servers
                    INNER JOIN channels ON channels.server_id = servers.id
                    WHERE channels.id = $1 LIMIT 1
                )
                SELECT users_servers.user_id
                FROM users_servers
                WHERE users_servers.user_id IS NOT NULL AND users_servers.server_id IN (SELECT server_id FROM server_query)
                "#,
                msg.channel.id,
            )
            .fetch_all(&db_pool)
            .await
            .unwrap();

            for user in &affected_users {
                if let Some(recipient) = users.get(&user.user_id) {
                    recipient
                        .do_send(SerializedWebSocketMessage::AddChannel(msg.channel.clone()))
                        .unwrap();
                }
            }
        }
        .into_actor(self)
        .wait(ctx)
    }
}

impl Handler<NewServer> for Server {
    type Result = ();

    fn handle(&mut self, msg: NewServer, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(recipient) = self.users.get(&msg.user_id) {
            recipient
                .do_send(SerializedWebSocketMessage::AddServer(
                    msg.server,
                    msg.channels,
                ))
                .unwrap();
        }
    }
}
