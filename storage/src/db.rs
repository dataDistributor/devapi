use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgRow, Row};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub id: i32,
    pub timestamp: DateTime<Utc>,
    pub request: Value,
    pub response: Value,
}

pub async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    // 1) Create the logs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS logs (
            id SERIAL PRIMARY KEY,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
            request JSONB NOT NULL,
            response JSONB NOT NULL
        );
        "#
    )
    .execute(pool)
    .await?;

    // 2) Create the credentials table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS credentials (
            id UUID PRIMARY KEY,
            name TEXT NOT NULL,
            token TEXT NOT NULL,
            nonce TEXT NOT NULL
        );
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_log(
    pool: &PgPool,
    request: Value,
    response: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO logs (request, response) VALUES ($1, $2)"
    )
    .bind(request)
    .bind(response)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_logs(pool: &PgPool) -> Result<Vec<ExecutionLog>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, timestamp, request, response
         FROM logs
         ORDER BY timestamp DESC
         LIMIT 100"
    )
    .map(|row: PgRow| ExecutionLog {
        id: row.get("id"),
        timestamp: row.get("timestamp"),
        request: row.get("request"),
        response: row.get("response"),
    })
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
