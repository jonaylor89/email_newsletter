use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use askama::Template;
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
    email_templates::{
        AlreadySubscribedEmailHtml, AlreadySubscribedEmailText, ConfirmationEmailHtml,
        ConfirmationEmailText,
    },
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;

        Ok(NewSubscriber { email, name })
    }
}

pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;

    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by: {}", cause)?;
        current = cause.source();
    }

    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber: NewSubscriber =
        form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    // Check if subscriber already exists
    let existing_subscriber = get_subscriber_by_email(&mut transaction, &new_subscriber.email)
        .await
        .context("Failed to check for existing subscriber")?;

    match existing_subscriber {
        Some((subscriber_id, status)) if status == "confirmed" => {
            // Already subscribed - send a friendly email
            transaction
                .commit()
                .await
                .context("Failed to commit SQL transaction")?;

            send_already_subscribed_email(&email_client, &new_subscriber)
                .await
                .context("Failed to send already-subscribed email")?;

            return Ok(HttpResponse::Ok().finish());
        }
        Some((subscriber_id, _)) => {
            // Pending confirmation - generate new token and resend
            let subscription_token = generate_subscription_token();

            store_token(&mut transaction, subscriber_id, &subscription_token)
                .await
                .context("Failed to store the confirmation token for existing subscriber")?;

            transaction
                .commit()
                .await
                .context("Failed to commit SQL transaction to store token")?;

            send_confirmation_email(
                &email_client,
                new_subscriber,
                &base_url.0,
                &subscription_token,
            )
            .await
            .context("Failed to send a confirmation email")?;

            return Ok(HttpResponse::Ok().finish());
        }
        None => {
            // New subscriber - proceed with insertion
            let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
                .await
                .context("Failed to insert new subcriber in the database")?;

            let subscription_token = generate_subscription_token();

            store_token(&mut transaction, subscriber_id, &subscription_token)
                .await
                .context("Failed to store the confirmation token for a new subscriber")?;

            transaction
                .commit()
                .await
                .context("Failed to commit SQL transaction to store a new subscriber")?;

            send_confirmation_email(
                &email_client,
                new_subscriber,
                &base_url.0,
                &subscription_token,
            )
            .await
            .context("Failed to send a confirmation email")?;

            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(name = "Check if subscriber exists by email", skip(transaction, email))]
pub async fn get_subscriber_by_email(
    transaction: &mut Transaction<'_, Postgres>,
    email: &SubscriberEmail,
) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT id, status
        FROM subscriptions
        WHERE email = $1
        "#,
        email.as_ref(),
    )
    .fetch_optional(transaction.as_mut())
    .await
    .map_err(|e| {
        tracing::info!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|row| (row.id, row.status)))
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(transaction.as_mut())
    .await
    .map_err(|e| {
        tracing::info!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO subscription_tokens (subscription_token, subscriber_id)
            VALUES ($1, $2)
            "#,
        subscription_token,
        subscriber_id,
    )
    .execute(transaction.as_mut())
    .await
    .map_err(|e| {
        tracing::info!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token,)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token,
    );

    let html_template = ConfirmationEmailHtml {
        subscriber_name: new_subscriber.name.as_ref().to_string(),
        confirmation_link: confirmation_link.clone(),
    };

    let text_template = ConfirmationEmailText {
        subscriber_name: new_subscriber.name.as_ref().to_string(),
        confirmation_link,
    };

    let html_body = html_template
        .render()
        .expect("Failed to render HTML email template");
    let plain_body = text_template
        .render()
        .expect("Failed to render text email template");

    email_client
        .send_email(
            &new_subscriber.email,
            "Confirm Your Subscription",
            &html_body,
            &plain_body,
        )
        .await
}

#[tracing::instrument(name = "Send already-subscribed email", skip(email_client, subscriber))]
pub async fn send_already_subscribed_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
) -> Result<(), reqwest::Error> {
    let html_template = AlreadySubscribedEmailHtml {
        subscriber_name: subscriber.name.as_ref().to_string(),
    };

    let text_template = AlreadySubscribedEmailText {
        subscriber_name: subscriber.name.as_ref().to_string(),
    };

    let html_body = html_template
        .render()
        .expect("Failed to render HTML email template");
    let plain_body = text_template
        .render()
        .expect("Failed to render text email template");

    email_client
        .send_email(
            &subscriber.email,
            "Already Subscribed",
            &html_body,
            &plain_body,
        )
        .await
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
