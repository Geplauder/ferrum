use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

#[tracing::instrument(name = "Get channels for server", skip(server_id, pool))]
pub async fn get_channels_for_server(
    server_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<ChannelModel>, sqlx::Error> {
    sqlx::query_as!(
        ChannelModel,
        r#"
        SELECT *
        FROM channels
        WHERE channels.server_id = $1
        "#,
        server_id
    )
    .fetch_all(pool)
    .await
}

#[tracing::instrument(name = "Get channels for user", skip(user_id, pool))]
pub async fn get_channels_for_user(user_id: Uuid, pool: &PgPool) -> Result<Vec<ChannelModel>, sqlx::Error> {
    sqlx::query_as!(
        ChannelModel,
        r#"
        WITH server_query AS (SELECT servers.id as server_id
            FROM users_servers
            INNER JOIN servers ON servers.id = users_servers.server_id
            WHERE users_servers.user_id = $1
        )
        SELECT channels.*
        FROM channels
        WHERE channels.server_id IN (SELECT server_id FROM server_query)
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}

#[tracing::instrument(
    name = "Saving a new server channel to the database",
    skip(transaction, new_channel, server_id)
)]
pub async fn insert_channel(
    transaction: &mut Transaction<'_, Postgres>,
    new_channel: &NewChannel,
    server_id: Uuid,
) -> Result<ChannelModel, sqlx::Error> {
    sqlx::query_as!(
        ChannelModel,
        r#"
        INSERT INTO channels (id, server_id, name)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        Uuid::new_v4(),
        server_id,
        new_channel.name.as_ref(),
    )
    .fetch_one(transaction)
    .await
}
