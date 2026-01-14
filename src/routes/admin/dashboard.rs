use anyhow::Context;
use askama::Template;
use axum::extract::State;
use axum::response::Html;
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::AuthenticatedUser;
use crate::utils::e500;
use crate::web_templates::AdminDashboardTemplate;

pub async fn admin_dashboard(
    AuthenticatedUser(user_id): AuthenticatedUser,
    State(pool): State<PgPool>,
) -> Result<Html<String>, crate::utils::AppError> {
    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    let template = AdminDashboardTemplate { username };

    Ok(Html(template.render().unwrap()))
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
