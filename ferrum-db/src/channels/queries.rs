use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

#[tracing::instrument(name = "Get channel with id", skip(channel_id, pool))]
pub async fn get_channel_with_id(
    channel_id: Uuid,
    pool: &PgPool,
) -> Result<ChannelModel, sqlx::Error> {
    sqlx::query_as!(
        ChannelModel,
        r#"
        SELECT *
        FROM channels
        WHERE channels.id = $1
        "#,
        channel_id,
    )
    .fetch_one(pool)
    .await
}

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

#[tracing::instrument(
    name = "Update a existing channels' name",
    skip(pool, channel_id, name)
)]
pub async fn update_channel_name(
    pool: &PgPool,
    channel_id: Uuid,
    name: &ChannelName,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE channels
        SET name = $1
        WHERE id = $2
        "#,
        name.as_ref(),
        channel_id,
    )
    .execute(pool)
    .await?;

    Ok(())
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

#[tracing::instrument(name = "Delete a existing channel from the database")]
pub async fn delete_channel(
    transaction: &mut Transaction<'_, Postgres>,
    channel_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM channels
        WHERE channels.id = $1
        "#,
        channel_id
    )
    .execute(transaction)
    .await?;

    Ok(())
}
