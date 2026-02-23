use std::sync::Once;
use std::time::Duration;

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use sqlx::{Column, Row, TypeInfo};

static INSTALL_DRIVERS: Once = Once::new();

/// Execute a SQL query against the given connection URL and return the result rows as JSON objects.
pub async fn execute_query(
    connection_url: &str,
    query: &str,
    params: &[Value],
    read_only: bool,
    max_rows: usize,
    timeout: Duration,
) -> Result<Vec<Map<String, Value>>> {
    INSTALL_DRIVERS.call_once(|| {
        sqlx::any::install_default_drivers();
    });

    // For SQLite, enforce read-only mode via the connection URL before connecting.
    let connection_url: std::borrow::Cow<'_, str> = if read_only
        && connection_url.to_ascii_lowercase().starts_with("sqlite")
        && !connection_url.to_ascii_lowercase().contains("mode=ro")
    {
        let separator = if connection_url.contains('?') {
            "&"
        } else {
            "?"
        };
        std::borrow::Cow::Owned(format!("{connection_url}{separator}mode=ro"))
    } else {
        std::borrow::Cow::Borrowed(connection_url)
    };

    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(timeout)
        .connect(&connection_url)
        .await
        .context("failed connecting to SQL database")?;

    // For PostgreSQL and MySQL, enforce read-only mode via explicit read-only
    // transactions instead of SET commands (which can be overridden by users).
    // SQLite is already handled via mode=ro connection parameter above.
    let url_lower = connection_url.to_ascii_lowercase();
    let use_read_only_transaction =
        read_only && (url_lower.starts_with("postgres") || url_lower.starts_with("mysql"));

    if use_read_only_transaction {
        let begin_stmt = if url_lower.starts_with("postgres") {
            "BEGIN READ ONLY"
        } else {
            "START TRANSACTION READ ONLY"
        };
        sqlx::query(begin_stmt)
            .execute(&pool)
            .await
            .context("failed starting read-only transaction")?;
    }

    // codeql[rust/cleartext-storage-database] - False positive: the connection URL (which may
    // contain credentials) is used to *connect* to the database, not to store data in it.
    let mut sqlx_query = sqlx::query(query);
    for param in params {
        sqlx_query = bind_json_param(sqlx_query, param);
    }

    let query_result = tokio::time::timeout(timeout, sqlx_query.fetch_all(&pool))
        .await
        .map_err(|_| anyhow::anyhow!("SQL query timed out after {timeout:?}"));

    // Always rollback the read-only transaction, even on error.
    if use_read_only_transaction {
        let _ = sqlx::query("ROLLBACK").execute(&pool).await;
    }

    let rows = query_result?.context("SQL query execution failed")?;

    let mut results = Vec::with_capacity(rows.len().min(max_rows));
    for row in rows.iter().take(max_rows) {
        results.push(row_to_json(row)?);
    }

    pool.close().await;

    Ok(results)
}

fn bind_json_param<'q>(
    query: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>,
    value: &'q Value,
) -> sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>> {
    match value {
        Value::Null => query.bind(None::<String>),
        Value::Bool(b) => query.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else if let Some(f) = n.as_f64() {
                query.bind(f)
            } else {
                query.bind(n.to_string())
            }
        }
        Value::String(s) => query.bind(s.as_str()),
        _ => query.bind(serde_json::to_string(value).unwrap_or_default()),
    }
}

fn row_to_json(row: &sqlx::any::AnyRow) -> Result<Map<String, Value>> {
    let mut map = Map::new();
    for col in row.columns() {
        let name = col.name().to_string();
        let type_name = col.type_info().name();
        let value = match type_name {
            "INTEGER" | "INT" | "INT4" | "INT8" | "BIGINT" => row
                .try_get::<i64, _>(col.ordinal())
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null),
            "REAL" | "FLOAT" | "FLOAT4" | "FLOAT8" | "DOUBLE" | "NUMERIC" => row
                .try_get::<f64, _>(col.ordinal())
                .ok()
                .and_then(|v| serde_json::Number::from_f64(v).map(Value::Number))
                .unwrap_or(Value::Null),
            "BOOLEAN" | "BOOL" => row
                .try_get::<bool, _>(col.ordinal())
                .map(Value::Bool)
                .unwrap_or(Value::Null),
            _ => {
                // For unknown types (e.g. SQLite "NULL" for literal expressions),
                // try decoding as each type in order: i64 -> f64 -> bool -> String.
                let ordinal = col.ordinal();
                if let Ok(v) = row.try_get::<i64, _>(ordinal) {
                    Value::Number(v.into())
                } else if let Ok(v) = row.try_get::<f64, _>(ordinal) {
                    serde_json::Number::from_f64(v)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                } else if let Ok(v) = row.try_get::<bool, _>(ordinal) {
                    Value::Bool(v)
                } else if let Ok(v) = row.try_get::<String, _>(ordinal) {
                    Value::String(v)
                } else {
                    Value::Null
                }
            }
        };
        map.insert(name, value);
    }
    Ok(map)
}
