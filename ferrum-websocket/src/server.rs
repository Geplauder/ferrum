use std::collections::HashMap;

use anyhow::Context as AnyhowContext;
use async_trait::async_trait;
use ferrum_db::{
    channels::queries::{get_channel_with_id, get_channels_for_server, get_channels_for_user},
    messages::queries::get_message_with_id,
    servers::queries::{get_server_with_id, get_servers_for_user},
    users::queries::{get_user_with_id, get_users_for_channel, get_users_on_server},
};
use ferrum_shared::{
    broker::BrokerEvent, channels::ChannelResponse, servers::ServerResponse, users::UserResponse,
};
use meio::{ActionHandler, Actor, Address, Context, StartedBy, System};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{messages::UserStartsTyping, WebSocketSession};

use super::messages::{IdentifyUser, WebSocketClose, WebSocketSessionMessage};

///
/// Manages all [`crate::WebSocketSession`] and updates them with appropriate events.
///
pub struct WebSocketServer {
    pub db_pool: PgPool,
    pub users: HashMap<Uuid, Address<WebSocketSession>>,
}

impl WebSocketServer {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            users: HashMap::new(),
        }
    }

    fn identify_user(&mut self, user_id: Uuid, recipient: Address<WebSocketSession>) {
        self.users.insert(user_id, recipient);
    }

    fn close_user(&mut self, user_id: Uuid) {
        self.users.remove(&user_id);
    }

    #[tracing::instrument(
        name = "Handle new message broker event",
        skip(self, channel_id, message_id)
    )]
    pub async fn new_message(&mut self, channel_id: Uuid, message_id: Uuid) -> anyhow::Result<()> {
        // Get the message and message author and transform them to a response
        let message = get_message_with_id(message_id, &self.db_pool)
            .await
            .context("Error while fetching message")?;

        let user = get_user_with_id(message.user_id, &self.db_pool)
            .await
            .context("Error while fetching user")?;

        let message_response = message.to_response(user);

        // Get all users that should be notified about the new message and send it to them
        let affected_users = get_users_for_channel(channel_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::AddMessage(
                        message_response.clone(),
                    ))
                    .await
                    .context(format!(
                        "Error while sending message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "Handle new channel broker event", skip(self, channel_id))]
    async fn new_channel(&mut self, channel_id: Uuid) -> anyhow::Result<()> {
        // Get the channel and transform it into a response
        let channel: ChannelResponse = get_channel_with_id(channel_id, &self.db_pool)
            .await
            .context("Error while fetching channel")?
            .into();

        // Get all users that should be notified about the new channel and send it to them
        let affected_users = get_users_on_server(channel.server_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::AddChannel(channel.clone()))
                    .await
                    .context(format!(
                        "Error while sending message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(
        name = "Handle new server broker event",
        skip(self, user_id, server_id)
    )]
    async fn new_server(&mut self, user_id: Uuid, server_id: Uuid) -> anyhow::Result<()> {
        // Get the server and transform it into a response
        let server: ServerResponse = get_server_with_id(server_id, &self.db_pool)
            .await
            .context("Error while fetching server")?
            .into();

        // Get all channels and users of this server and transform them into proper responses
        let channels: Vec<ChannelResponse> = get_channels_for_server(server_id, &self.db_pool)
            .await
            .context("Error while fetching channels")?
            .iter()
            .map(|x| x.clone().into())
            .collect();

        let users_on_server: Vec<UserResponse> = get_users_on_server(server_id, &self.db_pool)
            .await
            .context("Error while fetching users on server")?
            .iter()
            .map(|x| x.clone().into())
            .collect();

        // Send them all to the new servers' owner
        if let Some(recipient) = self.users.get_mut(&user_id) {
            recipient
                .act(WebSocketSessionMessage::AddServer(
                    server.clone(),
                    channels,
                    users_on_server,
                ))
                .await
                .context(format!(
                    "Error while sending message to recipient: {:?}",
                    user_id
                ))?;
        }

        Ok(())
    }

    #[tracing::instrument(name = "Handle new user broker event", skip(self, user_id, server_id))]
    async fn new_user(&mut self, user_id: Uuid, server_id: Uuid) -> anyhow::Result<()> {
        // Get the new user and transform them into a response
        let new_user: UserResponse = get_user_with_id(user_id, &self.db_pool)
            .await
            .context("Error while fetching user")?
            .into();

        // Get all users that should be notified about the new user and send it to them
        let affected_users = get_users_on_server(server_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            // Don't send the new user to the new user itself
            if user.id == user_id {
                continue;
            }

            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::AddUser(
                        server_id,
                        new_user.clone(),
                    ))
                    .await
                    .context(format!(
                        "Error while sending message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "Handle user left broker event", skip(self, user_id, server_id))]
    async fn user_left(&mut self, user_id: Uuid, server_id: Uuid) -> anyhow::Result<()> {
        if let Some(recipient) = self.users.get_mut(&user_id) {
            recipient
                .act(WebSocketSessionMessage::DeleteServer(server_id))
                .await
                .context(format!(
                    "Error while sending delete server message to recipient: {:?}",
                    user_id
                ))?;
        }

        // Get all users that are on the server and notify them about the leaving user
        let affected_users = get_users_on_server(server_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            if user.id == user_id {
                continue;
            }

            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::DeleteUser(user_id, server_id))
                    .await
                    .context(format!(
                        "Error while sending delete user message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(
        name = "Handle user joined broker event",
        skip(self, user_id, server_id)
    )]
    async fn user_joined(&mut self, user_id: Uuid, server_id: Uuid) -> anyhow::Result<()> {
        // Send the new server to the joining user
        self.new_server(user_id, server_id).await?;

        // Send the new user to the existing users on the server
        self.new_user(user_id, server_id).await?;

        Ok(())
    }

    #[tracing::instrument(name = "Handle delete server broker event", skip(self, server_id))]
    async fn delete_server(&mut self, server_id: Uuid) -> anyhow::Result<()> {
        // Send the deleted server to all websocket sessions, letting them reject it if necessary
        for (user_id, recipient) in &mut self.users {
            recipient
                .act(WebSocketSessionMessage::DeleteServer(server_id))
                .await
                .context(format!(
                    "Error while sending message to recipient: {:?}",
                    user_id
                ))?;
        }

        Ok(())
    }

    #[tracing::instrument(
        name = "Handle delete channel broker event",
        skip(self, server_id, channel_id)
    )]
    async fn delete_channel(&mut self, server_id: Uuid, channel_id: Uuid) -> anyhow::Result<()> {
        for (user_id, recipient) in &mut self.users {
            recipient
                .act(WebSocketSessionMessage::DeleteChannel(
                    server_id, channel_id,
                ))
                .await
                .context(format!(
                    "Error while sending message to recipient: {:?}",
                    user_id
                ))?;
        }

        Ok(())
    }

    #[tracing::instrument(name = "Handle update server broker event", skip(self, server_id))]
    async fn update_server(&mut self, server_id: Uuid) -> anyhow::Result<()> {
        // Get updated server response
        let server: ServerResponse = get_server_with_id(server_id, &self.db_pool)
            .await
            .context("Error while fetching server")?
            .into();

        // Get all users that are on the server and notify them about the updated server
        let affected_users = get_users_on_server(server_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::UpdateServer(server.clone()))
                    .await
                    .context(format!(
                        "Error while sending message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(name = "Handle update channel broker event", skip(self, channel_id))]
    async fn update_channel(&mut self, channel_id: Uuid) -> anyhow::Result<()> {
        // Get updated channel response
        let channel: ChannelResponse = get_channel_with_id(channel_id, &self.db_pool)
            .await
            .context("Error while feteching channel")?
            .into();

        // Get all users that have access to the channel and notify them about the updated channel
        let affected_users = get_users_for_channel(channel_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::UpdateChannel(channel.clone()))
                    .await
                    .context(format!(
                        "Error while sending message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl StartedBy<System> for WebSocketServer {
    async fn handle(&mut self, _ctx: &mut Context<Self>) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

impl Actor for WebSocketServer {
    type GroupBy = ();
}

#[async_trait]
impl ActionHandler<IdentifyUser> for WebSocketServer {
    #[tracing::instrument(name = "Handle identify user websocket message", skip(self, _ctx))]
    async fn handle(
        &mut self,
        mut msg: IdentifyUser,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        // Store the user
        self.identify_user(msg.user_id, msg.addr.clone());

        let user_id = msg.user_id;

        // Get all servers for this user and inform their websocket session about them
        let servers = get_servers_for_user(user_id, &self.db_pool)
            .await
            .context("Error while fetching servers for user")?;

        let channels = get_channels_for_user(user_id, &self.db_pool)
            .await
            .context("Error while fetching channels for user")?;

        msg.addr
            .act(WebSocketSessionMessage::Ready(
                servers.iter().map(|x| x.id).collect(),
                channels.iter().map(|x| x.id).collect(),
            ))
            .await
            .context("Error while sending message to recipient")?;

        Ok(())
    }
}

#[async_trait]
impl ActionHandler<UserStartsTyping> for WebSocketServer {
    #[tracing::instrument(name = "Handle identify user websocket message", skip(self, _ctx))]
    async fn handle(
        &mut self,
        msg: UserStartsTyping,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        let typing_user: UserResponse = get_user_with_id(msg.user_id, &self.db_pool)
            .await
            .context("Error while fetching user")?
            .into();

        let affected_users = get_users_for_channel(msg.channel_id, &self.db_pool)
            .await
            .context("Error while fetching affected users")?;

        for user in &affected_users {
            if user.id == msg.user_id {
                continue;
            }

            if let Some(recipient) = self.users.get_mut(&user.id) {
                recipient
                    .act(WebSocketSessionMessage::UserStartsTyping(
                        typing_user.clone(),
                        msg.channel_id,
                    ))
                    .await
                    .context(format!(
                        "Error while sending message to recipient: {:?}",
                        user.id
                    ))?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ActionHandler<WebSocketClose> for WebSocketServer {
    #[tracing::instrument(name = "Handle websocket close message", skip(self, _ctx))]
    async fn handle(
        &mut self,
        msg: WebSocketClose,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        self.close_user(msg.user_id);

        Ok(())
    }
}

#[async_trait]
impl ActionHandler<BrokerEvent> for WebSocketServer {
    #[tracing::instrument(name = "Handle new incoming broker event", skip(self, _ctx), fields(request_id = %uuid::Uuid::new_v4()))]
    async fn handle(
        &mut self,
        msg: BrokerEvent,
        _ctx: &mut Context<Self>,
    ) -> Result<(), anyhow::Error> {
        match msg {
            BrokerEvent::NewChannel { channel_id } => self.new_channel(channel_id).await?,
            BrokerEvent::NewServer { user_id, server_id } => {
                self.new_server(user_id, server_id).await?
            }
            BrokerEvent::UserLeft { user_id, server_id } => {
                self.user_left(user_id, server_id).await?
            }
            BrokerEvent::UserJoined { user_id, server_id } => {
                self.user_joined(user_id, server_id).await?
            }
            BrokerEvent::DeleteServer { server_id } => self.delete_server(server_id).await?,
            BrokerEvent::DeleteChannel {
                server_id,
                channel_id,
            } => self.delete_channel(server_id, channel_id).await?,
            BrokerEvent::UpdateServer { server_id } => self.update_server(server_id).await?,
            BrokerEvent::UpdateChannel { channel_id } => self.update_channel(channel_id).await?,
            BrokerEvent::NewMessage {
                channel_id,
                message_id,
            } => self.new_message(channel_id, message_id).await?,
        }

        Ok(())
    }
}
