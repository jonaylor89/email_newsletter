use axum::body::Body;
use axum::response::Response;
use http::StatusCode;
use http_body_util::BodyExt;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

#[allow(clippy::large_enum_variant)]
pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT 
            response_status_code as "response_status_code!",
            response_headers AS "response_headers!: Vec<HeaderPairRecord>",
            response_body AS "response_body!"
        FROM idempotency 
        WHERE
            user_id = $1
        AND
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;
        let mut builder = http::Response::builder().status(status_code);
        for HeaderPairRecord { name, value } in r.response_headers {
            builder = builder.header(name, value);
        }
        let response = builder
            .body(Body::from(r.response_body))
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(Some(response.into()))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    http_response: Response,
) -> Result<Response, anyhow::Error> {
    let (response_head, body) = http_response.into_parts();

    let body = body
        .collect()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .to_bytes();
    let status_code = response_head.status.as_u16() as i16;

    let headers = {
        let mut h = Vec::with_capacity(response_head.headers.len());
        for (name, value) in response_head.headers.iter() {
            h.push(HeaderPairRecord {
                name: name.as_str().to_owned(),
                value: value.as_bytes().to_owned(),
            });
        }

        h
    };

    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency 
        SET 
            response_status_code = $3, 
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1
        AND 
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref(),
    )
    .execute(transaction.as_mut())
    .await?;
    transaction.commit().await?;

    let http_response = Response::from_parts(response_head, Body::from(body.clone()));
    Ok(http_response)
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let n_inserted_rows = sqlx::query_unchecked!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        ) VALUES (
            $1,
            $2,
            now()
        ) ON CONFLICT DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref(),
    )
    .execute(transaction.as_mut())
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No saved response found"))?;

        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
