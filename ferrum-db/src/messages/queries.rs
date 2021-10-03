use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

#[tracing::instrument(name = "Get messages from channel", skip(channel_id, pool))]
pub async fn get_messages_for_channel(
    channel_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<MessageModel>, sqlx::Error> {
    sqlx::query_as!(
        MessageModel,
        r#"
        SELECT *
        FROM messages
        WHERE messages.channel_id = $1
        "#,
        channel_id,
    )
    .fetch_all(pool)
    .await
}

#[tracing::instrument(
    name = "Saving a new channel message to the database",
    skip(transaction, new_message, channel_id, user_id)
)]
pub async fn insert_message(
    transaction: &mut Transaction<'_, Postgres>,
    pool: &PgPool,
    new_message: &NewMessage,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<MessageModel, sqlx::Error> {
    sqlx::query_as!(
        MessageModel,
        r#"
        INSERT INTO messages (id, channel_id, user_id, content)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        Uuid::new_v4(),
        channel_id,
        user_id,
        new_message.content.as_ref(),
    )
    .fetch_one(transaction)
    .await
}
