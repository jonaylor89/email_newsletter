use std::time::Duration;

use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::task::JoinSet;
use tracing::{field::display, Span};
use uuid::Uuid;

use crate::{
    configuration::Settings, domain::SubscriberEmail, email_client::EmailClient,
    startup::get_connection_pool,
};

// Number of tasks to process concurrently
const CONCURRENT_TASKS: usize = 10;

// Maximum number of retry attempts before moving to dead letter queue
const MAX_RETRY_ATTEMPTS: i32 = 5;

// Minimum time between retry attempts (exponential backoff base)
const RETRY_BACKOFF_MINUTES: i64 = 5;

type PgTransaction = Transaction<'static, Postgres>;

struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();
    worker_loop(&connection_pool, &email_client).await
}

async fn worker_loop(pool: &PgPool, email_client: &EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_tasks(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn try_execute_tasks(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    // Dequeue multiple tasks at once
    let tasks = dequeue_tasks(pool, CONCURRENT_TASKS).await?;

    if tasks.is_empty() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }

    let task_count = tasks.len();
    tracing::info!("Processing {} tasks concurrently", task_count);

    // Process tasks concurrently using JoinSet
    let mut join_set = JoinSet::new();

    for (transaction, issue_id, email) in tasks {
        let pool_clone = pool.clone();
        let email_client_clone = email_client.clone();

        join_set.spawn(async move {
            execute_single_task(pool_clone, email_client_clone, transaction, issue_id, email).await
        });
    }

    // Wait for all tasks to complete
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                tracing::error!(
                    error.cause_chain = ?e,
                    "Task execution failed"
                );
            }
            Err(e) => {
                tracing::error!(
                    error.cause_chain = ?e,
                    "Task join failed"
                );
            }
        }
    }

    Ok(ExecutionOutcome::TaskCompleted)
}

#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty,
    )
)]
async fn execute_single_task(
    pool: PgPool,
    email_client: EmailClient,
    transaction: PgTransaction,
    issue_id: Uuid,
    email: String,
) -> Result<(), anyhow::Error> {
    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&email));

    // Get current attempt count
    let attempt_count = get_attempt_count(&pool, issue_id, &email).await?;

    // Check if we should retry this task based on exponential backoff
    if let Some(last_attempted) = get_last_attempted(&pool, issue_id, &email).await? {
        let backoff_duration = Duration::from_secs(
            (RETRY_BACKOFF_MINUTES * 60 * 2_i64.pow(attempt_count as u32).min(32)) as u64,
        );
        let elapsed = Utc::now() - last_attempted;

        if elapsed < chrono::Duration::from_std(backoff_duration).unwrap() {
            // Too soon to retry - skip this task for now
            tracing::debug!(
                "Skipping task (backoff): attempt {}, last_attempted {:?} ago",
                attempt_count,
                elapsed
            );
            // Just rollback transaction without deleting
            transaction.rollback().await?;
            return Ok(());
        }
    }

    let send_result = match SubscriberEmail::parse(email.clone()) {
        Ok(email_addr) => {
            let issue = get_issue(&pool, issue_id).await?;
            email_client
                .send_email(
                    &email_addr,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
        }
        Err(e) => {
            // Invalid email - this is a non-retryable error
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Invalid email address - moving to dead letter queue"
            );
            move_to_dead_letter_queue(&pool, issue_id, &email, attempt_count, &e.to_string())
                .await?;
            delete_task(transaction, issue_id, &email).await?;
            return Ok(());
        }
    };

    match send_result {
        Ok(_) => {
            // Success - delete from queue
            tracing::info!("Successfully sent email to {}", email);
            delete_task(transaction, issue_id, &email).await?;
        }
        Err(e) => {
            let error_message = e.to_string();
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %error_message,
                attempt = attempt_count + 1,
                "Failed to deliver issue to confirmed subscriber"
            );

            // Update attempt count and error message
            let new_attempt_count = attempt_count + 1;

            if new_attempt_count >= MAX_RETRY_ATTEMPTS {
                // Max retries reached - move to dead letter queue
                tracing::warn!(
                    "Max retry attempts ({}) reached for {}. Moving to dead letter queue.",
                    MAX_RETRY_ATTEMPTS,
                    email
                );
                move_to_dead_letter_queue(
                    &pool,
                    issue_id,
                    &email,
                    new_attempt_count,
                    &error_message,
                )
                .await?;
                delete_task(transaction, issue_id, &email).await?;
            } else {
                // Update retry tracking and keep in queue
                update_retry_tracking(&pool, issue_id, &email, new_attempt_count, &error_message)
                    .await?;
                transaction.rollback().await?;
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn dequeue_tasks(
    pool: &PgPool,
    limit: usize,
) -> Result<Vec<(PgTransaction, Uuid, String)>, anyhow::Error> {
    let mut tasks = Vec::new();

    // Dequeue tasks one by one to get separate transactions for each
    // This allows parallel processing without holding locks
    for _ in 0..limit {
        let mut transaction = pool.begin().await?;
        let r = sqlx::query!(
            r#"
            SELECT newsletter_issue_id, subscriber_email
            FROM issue_delivery_queue
            FOR UPDATE
            SKIP LOCKED
            LIMIT 1
            "#,
        )
        .fetch_optional(transaction.as_mut())
        .await?;

        if let Some(r) = r {
            tasks.push((transaction, r.newsletter_issue_id, r.subscriber_email));
        } else {
            // No more tasks available
            break;
        }
    }

    Ok(tasks)
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE 
            newsletter_issue_id = $1 
        AND
            subscriber_email = $2
        "#,
        issue_id,
        email,
    )
    .execute(transaction.as_mut())
    .await?;

    transaction.commit().await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let issue: NewsletterIssue = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE
            newsletter_issue_id = $1
        "#,
        issue_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(issue)
}

#[tracing::instrument(skip_all)]
async fn get_attempt_count(
    pool: &PgPool,
    issue_id: Uuid,
    email: &str,
) -> Result<i32, anyhow::Error> {
    let result = sqlx::query!(
        r#"
        SELECT attempt_count
        FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email,
    )
    .fetch_one(pool)
    .await?;

    Ok(result.attempt_count)
}

#[tracing::instrument(skip_all)]
async fn get_last_attempted(
    pool: &PgPool,
    issue_id: Uuid,
    email: &str,
) -> Result<Option<chrono::DateTime<Utc>>, anyhow::Error> {
    let result = sqlx::query!(
        r#"
        SELECT last_attempted_at
        FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email,
    )
    .fetch_one(pool)
    .await?;

    Ok(result.last_attempted_at)
}

#[tracing::instrument(skip_all)]
async fn update_retry_tracking(
    pool: &PgPool,
    issue_id: Uuid,
    email: &str,
    attempt_count: i32,
    error_message: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        UPDATE issue_delivery_queue
        SET attempt_count = $3,
            last_attempted_at = $4,
            error_message = $5
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email,
        attempt_count,
        Utc::now(),
        error_message,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn move_to_dead_letter_queue(
    pool: &PgPool,
    issue_id: Uuid,
    email: &str,
    attempt_count: i32,
    error_message: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        INSERT INTO dead_letter_queue (
            newsletter_issue_id,
            subscriber_email,
            attempt_count,
            last_error,
            failed_at
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (newsletter_issue_id, subscriber_email)
        DO UPDATE SET
            attempt_count = $3,
            last_error = $4,
            failed_at = $5
        "#,
        issue_id,
        email,
        attempt_count,
        error_message,
        Utc::now(),
    )
    .execute(pool)
    .await?;

    Ok(())
}
