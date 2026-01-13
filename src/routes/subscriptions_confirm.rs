use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::SubscriptionToken;

#[derive(serde::Deserialize)]
pub struct Parameters {
    pub subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    Query(parameters): Query<Parameters>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    // Validate token format before querying database
    let token = match SubscriptionToken::parse(parameters.subscription_token.clone()) {
        Ok(token) => token,
        Err(e) => {
            tracing::warn!("Invalid token format: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                "Invalid confirmation token format. The token must be 25 alphanumeric characters.",
            );
        }
    };

    let id = match get_subscriber_id_from_token(&pool, token.as_ref()).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to get subscriber ID from token: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to confirm subscription. Please try again later.",
            );
        }
    };

    match id {
        None => {
            // Token doesn't exist or is invalid
            tracing::warn!(
                "Non-existent confirmation token: {}",
                parameters.subscription_token
            );
            return (
                StatusCode::BAD_REQUEST,
                "Invalid confirmation token. The token may have expired or does not exist.",
            );
        }
        Some(subscriber_id) => match confirm_subscriber(&pool, subscriber_id).await {
            Ok(_) => (StatusCode::OK, "Your subscription has been confirmed!"),
            Err(e) => {
                tracing::error!("Failed to confirm subscriber {}: {:?}", subscriber_id, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to confirm subscription. Please try again later.",
                )
            }
        },
    }
}

#[tracing::instrument(name = "Get subscriber status", skip(subscriber_id, pool))]
pub async fn get_subscriber_status(
    pool: &PgPool,
    subscriber_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
            SELECT status
            FROM subscriptions
            WHERE id = $1
        "#,
        subscriber_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.status))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    // Check current status to make operation idempotent
    let current_status = get_subscriber_status(pool, subscriber_id).await?;

    match current_status {
        Some(status) if status == "confirmed" => {
            // Already confirmed - idempotent operation
            tracing::info!("Subscriber {} is already confirmed", subscriber_id);
            Ok(())
        }
        Some(_) => {
            // Pending confirmation - update to confirmed
            sqlx::query!(
                r#"
                UPDATE subscriptions
                SET status = 'confirmed'
                WHERE id = $1
                "#,
                subscriber_id,
            )
            .execute(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to execute query {:?}", e);
                e
            })?;

            Ok(())
        }
        None => {
            // Subscriber doesn't exist - this shouldn't happen
            tracing::error!("Subscriber {} not found", subscriber_id);
            Err(sqlx::Error::RowNotFound)
        }
    }
}

#[tracing::instrument(name = "Get subscriber id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
            SELECT subscriber_id
            FROM subscription_tokens
            WHERE subscription_token = $1
        "#,
        subscription_token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}
