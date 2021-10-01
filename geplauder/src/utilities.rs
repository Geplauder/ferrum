use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(
    name = "Check if user has access to a channel",
    skip(pool, channel_id, user_id)
)]
pub async fn does_user_have_access_to_channel(
    pool: &PgPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        WITH server_query AS (
            SELECT servers.id as server_id
            FROM servers
            INNER JOIN channels ON channels.server_id = servers.id
            WHERE channels.id = $1 LIMIT 1
        )
        SELECT users_servers.*
        FROM users_servers
        WHERE users_servers.user_id = $2 AND users_servers.server_id IN (SELECT server_id FROM server_query)
        "#,
        channel_id,
        user_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(name = "Check if user is on a server", skip(pool, user_id, server_id))]
pub async fn is_user_on_server(
    pool: &PgPool,
    user_id: Uuid,
    server_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT servers.id
        FROM users_servers
        INNER JOIN servers ON users_servers.server_id = servers.id
        WHERE users_servers.user_id = $1 AND users_servers.server_id = $2
        "#,
        user_id,
        server_id,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

pub async fn get_users_on_server(pool: &PgPool, server_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    let affected_users = sqlx::query!(
        r#"
        SELECT users_servers.user_id
        FROM users_servers
        WHERE users_servers.user_id IS NOT NULL AND users_servers.server_id = $1
        "#,
        server_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(affected_users.iter().map(|x| x.user_id).collect())
}
