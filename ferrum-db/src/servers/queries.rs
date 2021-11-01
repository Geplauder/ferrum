use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

// TODO: Find a better way for the "SELECT *" queries, maybe macros?

#[tracing::instrument(name = "Get server with id", skip(server_id, pool))]
pub async fn get_server_with_id(
    server_id: Uuid,
    pool: &PgPool,
) -> Result<ServerModel, sqlx::Error> {
    sqlx::query_as!(
        ServerModel,
        r#"
        SELECT id, name, owner_id, flags as "flags: ServerFlags", updated_at, created_at
        FROM servers
        WHERE servers.id = $1
        "#,
        server_id
    )
    .fetch_one(pool)
    .await
}

#[tracing::instrument(name = "Get server for channel id", skip(channel_id, pool))]
pub async fn get_server_for_channel_id(
    channel_id: Uuid,
    pool: &PgPool,
) -> Result<ServerModel, sqlx::Error> {
    sqlx::query_as!(
        ServerModel,
        r#"
            SELECT servers.id, servers.name, servers.owner_id, servers.flags as "flags: ServerFlags", servers.updated_at, servers.created_at
            FROM channels
            INNER JOIN servers ON channels.server_id = servers.id
            WHERE channels.id = $1
        "#,
        channel_id
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
        RETURNING id, name, owner_id, flags as "flags: ServerFlags", updated_at, created_at
        "#,
        Uuid::new_v4(),
        new_server.name.as_ref(),
        user_id,
    )
    .fetch_one(transaction)
    .await
}

#[tracing::instrument(name = "Delete a existing server from the database")]
pub async fn delete_server(
    transaction: &mut Transaction<'_, Postgres>,
    server_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        DELETE FROM servers
        WHERE servers.id = $1
        "#,
        server_id
    )
    .execute(transaction)
    .await?;

    Ok(())
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
    name = "Remove a users_servers entry from the database",
    skip(transaction, user_id, server_id)
)]
pub async fn remove_user_from_server(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM users_servers
        WHERE users_servers.user_id = $1 AND users_servers.server_id = $2
        "#,
        user_id,
        server_id,
    )
    .execute(transaction)
    .await?;

    Ok(query.rows_affected() > 0)
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

#[tracing::instrument(name = "Get servers for user", skip(user_id, pool))]
pub async fn get_servers_for_user(
    user_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<ServerModel>, sqlx::Error> {
    sqlx::query_as!(
        ServerModel,
        r#"
        SELECT servers.id, servers.name, servers.owner_id, servers.flags as "flags: ServerFlags", servers.updated_at, servers.created_at
        FROM users_servers
        INNER JOIN servers ON users_servers.server_id = servers.id
        WHERE users_servers.user_id = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}

#[tracing::instrument(name = "Update a existing servers' name", skip(pool, server_id, name))]
pub async fn update_server_name(
    pool: &PgPool,
    server_id: Uuid,
    name: &ServerName,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE servers
        SET name = $1
        WHERE id = $2
        "#,
        name.as_ref(),
        server_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}
