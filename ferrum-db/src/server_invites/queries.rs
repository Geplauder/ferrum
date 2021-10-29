use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

#[tracing::instrument(name = "Get server invite for code", skip(code, pool))]
pub async fn get_server_invite_with_code(
    code: &str,
    pool: &PgPool,
) -> Result<ServerInviteModel, sqlx::Error> {
    sqlx::query_as!(
        ServerInviteModel,
        r#"
        SELECT *
        FROM server_invites
        WHERE server_invites.code = $1
        "#,
        code
    )
    .fetch_one(pool)
    .await
}

#[tracing::instrument(name = "Get server invites for server", skip(server_id, pool))]
pub async fn get_server_invites_for_server(
    server_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<ServerInviteModel>, sqlx::Error> {
    sqlx::query_as!(
        ServerInviteModel,
        r#"
        SELECT *
        FROM server_invites
        WHERE server_invites.server_id = $1
        "#,
        server_id,
    )
    .fetch_all(pool)
    .await
}

#[tracing::instrument(
    name = "Save a new server invite to the database",
    skip(transaction, server_id, code)
)]
pub async fn insert_server_invite(
    transaction: &mut Transaction<'_, Postgres>,
    server_id: Uuid,
    code: String,
) -> Result<ServerInviteModel, sqlx::Error> {
    sqlx::query_as!(
        ServerInviteModel,
        r#"
        INSERT INTO server_invites (id, server_id, code)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        Uuid::new_v4(),
        server_id,
        code,
    )
    .fetch_one(transaction)
    .await
}
