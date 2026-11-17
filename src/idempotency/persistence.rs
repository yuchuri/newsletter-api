use axum::{
    body::{self, Body},
    http::StatusCode,
    response::Response,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::idempotency::IdempotencyKey;

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> anyhow::Result<NextAction> {
    let mut transaction = pool.begin().await?;
    let n_inserted_rows = sqlx::query!(
        r#"
        insert into idempotency (
            user_id,
            idempotency_key
        )
        values ($1, $2)
        on conflict do nothing
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut *transaction)
    .await?
    .rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> anyhow::Result<Option<Response>> {
    let saved_response = sqlx::query!(
        r#"
        select 
            response_status_code as "response_status_code!",
            response_headers as "response_headers!: Vec<HeaderPairRecord>",
            response_body as "response_body!"
        from idempotency
        where
            user_id = $1 and
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = saved_response {
        let status_code = StatusCode::from_u16(record.response_status_code.try_into()?)?;
        let mut response = Response::builder().status(status_code);
        for HeaderPairRecord { name, value } in record.response_headers {
            response = response.header(name, value);
        }
        Ok(Some(response.body(record.response_body.into())?))
    } else {
        Ok(None)
    }
}

pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: Response,
) -> anyhow::Result<Response> {
    let (response_head, body) = response.into_parts();
    let body = body::to_bytes(body, usize::MAX).await?;
    let status_code = response_head.status.as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(response_head.headers.len());
        for (name, value) in response_head.headers.iter() {
            h.push(HeaderPairRecord {
                name: name.to_string(),
                value: value.as_bytes().to_vec(),
            });
        }
        h
    };

    sqlx::query_unchecked!(
        r#"
        update idempotency
        set
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        where
            user_id = $1 and
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body.as_ref(),
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let response = Response::from_parts(response_head, Body::from(body));
    Ok(response)
}
