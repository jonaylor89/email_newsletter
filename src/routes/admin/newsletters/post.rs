use anyhow::Context;
use axum::extract::{Form, State};
use axum::response::Response;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    authentication::AuthenticatedUser,
    idempotency::{save_response, try_processing, IdempotencyKey, NextAction},
    session_state::TypedSession,
    utils::{e400, e500, see_other},
};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text: String,
    html: String,
    idempotency_key: String,
}

#[tracing::instrument(
    name = "Publish a newsletter issue", 
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn publish_newsletter(
    AuthenticatedUser(user_id): AuthenticatedUser,
    State(pool): State<PgPool>,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<Response, crate::utils::AppError> {
    let FormData {
        title,
        text,
        html,
        idempotency_key,
    } = form;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            session
                .flash_info("The newsletter issue has been accepted - emails will go out shortly")
                .await;
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text, &html)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(e500)?;

    enequeue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    session
        .flash_info("The newsletter issue has been accepted - emails will go out shortly")
        .await;
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;

    Ok(response)
}

#[tracing::instrument(skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text: &str,
    html: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
        )
        VALUES ($1, $2, $3, $4, NOW())
        "#,
        newsletter_issue_id,
        title,
        text,
        html,
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enequeue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
        )
        SELECT $1, email 
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        newsletter_issue_id,
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(())
}
