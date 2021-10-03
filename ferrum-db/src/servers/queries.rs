use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

#[tracing::instrument(name = "Get server with id", skip(server_id, pool))]
pub async fn get_server_with_id(
    server_id: Uuid,
    pool: &PgPool,
) -> Result<ServerModel, sqlx::Error> {
    sqlx::query_as!(
        ServerModel,
        r#"
        SELECT *
        FROM servers
        WHERE servers.id = $1
        "#,
        server_id
    )
    .fetch_one(pool)
    .await
}

#[tracing::instrument(
    name = "Saving a new server to the database",
    skip(transaction, new_server)
)]
pub async fn insert_server(
    transaction: &mut Transaction<'_, Postgres>,
    new_server: &NewServer,
    user_id: Uuid,
) -> Result<ServerModel, sqlx::Error> {
    sqlx::query_as!(
        ServerModel,
        r#"
        INSERT INTO servers (id, name, owner_id)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        Uuid::new_v4(),
        new_server.name.as_ref(),
        user_id,
    )
    .fetch_one(transaction)
    .await
}

#[tracing::instrument(
    name = "Saving a new users_servers entry to the database",
    skip(transaction, user_id, server_id)
)]
pub async fn add_user_to_server(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO users_servers (id, user_id, server_id) VALUES ($1, $2, $3)
        "#,
        Uuid::new_v4(),
        user_id,
        server_id,
    )
    .execute(transaction)
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving a new default server channel to the database",
    skip(transaction, server_id)
)]
pub async fn add_default_channel_to_server(
    transaction: &mut Transaction<'_, Postgres>,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO channels (id, server_id, name) VALUES ($1, $2, $3)
        "#,
        Uuid::new_v4(),
        server_id,
        "general",
    )
    .execute(transaction)
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Check if user is owner of the server",
    skip(pool, server_id, user_id)
)]
pub async fn is_user_owner_of_server(
    pool: &PgPool,
    server_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!("SELECT owner_id FROM servers WHERE id = $1", server_id)
        .fetch_one(pool)
        .await?;

    Ok(row.owner_id == user_id)
}
