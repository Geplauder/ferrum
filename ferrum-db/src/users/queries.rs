use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::models::*;

#[tracing::instrument(name = "Get stored user with email", skip(email, pool))]
pub async fn get_user_with_email(
    email: &str,
    pool: &PgPool,
) -> Result<Option<UserModel>, sqlx::Error> {
    sqlx::query_as!(
        UserModel,
        r#"
        SELECT *
        FROM users
        WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
}

#[tracing::instrument(name = "Get stored user with id", skip(user_id, pool))]
pub async fn get_user_with_id(user_id: Uuid, pool: &PgPool) -> Result<UserModel, sqlx::Error> {
    sqlx::query_as!(
        UserModel,
        r#"
        SELECT *
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
}

#[tracing::instrument(
    name = "Saving a new user to the database",
    skip(transaction, new_user)
)]
pub async fn insert_user(
    transaction: &mut Transaction<'_, Postgres>,
    new_user: &NewUser,
) -> Result<UserModel, sqlx::Error> {
    sqlx::query_as!(
        UserModel,
        r#"
        INSERT INTO users (id, username, email, password)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
        Uuid::new_v4(),
        new_user.name.as_ref(),
        new_user.email.as_ref(),
        new_user.password.as_ref()
    )
    .fetch_one(transaction)
    .await
}

#[tracing::instrument(name = "Get server users", skip(server_id, pool))]
pub async fn get_users_on_server(
    server_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<UserModel>, sqlx::Error> {
    sqlx::query_as!(
        UserModel,
        r#"
        SELECT users.*
        FROM users_servers
        INNER JOIN users ON users_servers.user_id = users.id
        WHERE users_servers.server_id = $1
        "#,
        server_id,
    )
    .fetch_all(pool)
    .await
}

#[tracing::instrument(name = "get users for channel", skip(channel_id, pool))]
pub async fn get_users_for_channel(
    channel_id: Uuid,
    pool: &PgPool,
) -> Result<Vec<UserModel>, sqlx::Error> {
    sqlx::query_as!(
        UserModel,
        r#"
        WITH server_query AS (
            SELECT servers.id as server_id
            FROM servers
            INNER JOIN channels ON channels.server_id = servers.id
            WHERE channels.id = $1
            LIMIT 1
        )
        SELECT users.*
        FROM users_servers
        INNER JOIN users ON users.id = users_servers.user_id
        WHERE users_servers.server_id IN (SELECT server_id FROM server_query)
        LIMIT 1
        "#,
        channel_id
    )
    .fetch_all(pool)
    .await
}

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

#[tracing::instrument(name = "Update a existing users' name", skip(pool, user_id, name))]
pub async fn update_user_name(
    pool: &PgPool,
    user_id: Uuid,
    name: &UserName,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET username = $1
        WHERE id = $2
        "#,
        name.as_ref(),
        user_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(name = "Update a existing users' email", skip(pool, user_id, email))]
pub async fn update_user_email(
    pool: &PgPool,
    user_id: Uuid,
    email: &UserEmail,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET email = $1
        WHERE id = $2
        "#,
        email.as_ref(),
        user_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Update a existing users' password",
    skip(pool, user_id, password)
)]
pub async fn update_user_password(
    pool: &PgPool,
    user_id: Uuid,
    password: &UserPassword,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET password = $1
        WHERE id = $2
        "#,
        password.as_ref(),
        user_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}
