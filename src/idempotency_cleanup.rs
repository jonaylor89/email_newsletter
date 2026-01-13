use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;

use crate::{configuration::Settings, startup::get_connection_pool};

// Retention period for idempotency keys (30 days)
const RETENTION_DAYS: i64 = 30;

// How often to run the cleanup (24 hours)
const CLEANUP_INTERVAL_HOURS: u64 = 24;

pub async fn run_cleanup_worker(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    cleanup_loop(&connection_pool).await
}

async fn cleanup_loop(pool: &PgPool) -> Result<(), anyhow::Error> {
    loop {
        match delete_stale_idempotency_keys(pool).await {
            Ok(deleted_count) => {
                tracing::info!("Deleted {} stale idempotency keys", deleted_count);
            }
            Err(e) => {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to delete stale idempotency keys"
                );
            }
        }

        // Sleep for the cleanup interval
        tokio::time::sleep(Duration::from_secs(CLEANUP_INTERVAL_HOURS * 3600)).await;
    }
}

#[tracing::instrument(skip_all)]
async fn delete_stale_idempotency_keys(pool: &PgPool) -> Result<u64, anyhow::Error> {
    let cutoff_date = Utc::now() - chrono::Duration::days(RETENTION_DAYS);

    let result = sqlx::query!(
        r#"
        DELETE FROM idempotency
        WHERE created_at < $1
        "#,
        cutoff_date,
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_period_is_30_days() {
        assert_eq!(RETENTION_DAYS, 30);
    }

    #[test]
    fn cleanup_runs_daily() {
        assert_eq!(CLEANUP_INTERVAL_HOURS, 24);
    }
}
