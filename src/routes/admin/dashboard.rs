use anyhow::Context;
use axum::extract::State;
use axum::response::Html;
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::AuthenticatedUser;
use crate::utils::e500;

pub async fn admin_dashboard(
    AuthenticatedUser(user_id): AuthenticatedUser,
    State(pool): State<PgPool>,
) -> Result<Html<String>, crate::utils::AppError> {
    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    Ok(Html(format!(
        r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html; charset=utf-8">
                <title>Admin dashboard</title>
            </head>
            <body>
                <p>Welcome, {username}!</p>
                <p>Available actions:</p>
                <ol>
                    <li><a href="/admin/password">Change password</a></li>
                    <li><a href="/admin/newsletters">Create a new newsletter</a></li>
                    <li>
                        <form name="logoutForm" action="/admin/logout" method="post">
                            <input type="submit" value="Logout">
                        </form>
                    </li>
                </ol>
            </body>
            </html>"#,
    )))
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform a query to retrieve a username")?;

    Ok(row.username)
}
