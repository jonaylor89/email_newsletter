use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction};
use tokio::task::JoinSet;
use tracing::{field::display, Span};
use uuid::Uuid;

use crate::{domain::SubscriberEmail, email_client::EmailClient, startup::get_connection_pool, configuration::Settings};

// Number of tasks to process concurrently
const CONCURRENT_TASKS: usize = 10;

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

pub async fn run_worker_until_stopped(
    configuration: Settings,
) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let email_client = configuration.email_client.client();
    worker_loop(&connection_pool, &email_client).await
}

async fn worker_loop(pool: &PgPool, email_client: &EmailClient) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_tasks(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            },
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
            Ok(Ok(())) => {},
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

    match SubscriberEmail::parse(email.clone()) {
        Ok(email_addr) => {
            let issue = get_issue(&pool, issue_id).await?;
            if let Err(e) = email_client
                .send_email(
                    &email_addr,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await
            {
                tracing::error!(
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Failed to deliver issue to confirmed subscriber. Skipping"
                )
            }
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Skipping a confirmed subscriber. Their stored contact details are invalid"
            )
        }
    }

    delete_task(transaction, issue_id, &email).await?;

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
        .fetch_optional(&mut transaction)
        .await?;

        if let Some(r) = r {
            tasks.push((
                transaction,
                r.newsletter_issue_id,
                r.subscriber_email,
            ));
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
    .execute(&mut transaction)
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
