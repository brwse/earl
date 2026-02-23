#![cfg(feature = "sql")]

use std::net::IpAddr;
use std::time::Duration;

use earl::protocol::builder::{PreparedProtocolData, PreparedRequest, PreparedSqlQuery};
use earl::protocol::executor::execute_prepared_request_with_host_validator;
use earl::protocol::transport::ResolvedTransport;
use earl::template::schema::{CommandMode, ResultDecode, ResultTemplate};
use earl_core::Redactor;
use serde_json::Map;

fn loopback_resolver() -> Vec<IpAddr> {
    vec![]
}

fn default_transport() -> ResolvedTransport {
    ResolvedTransport {
        timeout: Duration::from_secs(10),
        follow_redirects: false,
        max_redirect_hops: 0,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    }
}

fn prepared_sql_request(
    connection_url: &str,
    query: &str,
    params: Vec<serde_json::Value>,
    read_only: bool,
    max_rows: usize,
) -> PreparedRequest {
    PreparedRequest {
        key: "test.sql".to_string(),
        mode: CommandMode::Read,
        stream: false,
        allow_rules: vec![],
        transport: default_transport(),
        result_template: ResultTemplate {
            decode: ResultDecode::Json,
            extract: None,
            output: "{{ result }}".to_string(),
            result_alias: None,
        },
        args: Map::new(),
        redactor: Redactor::default(),
        protocol_data: PreparedProtocolData::Sql(PreparedSqlQuery {
            connection_url: connection_url.to_string(),
            query: query.to_string(),
            params,
            read_only,
            max_rows,
        }),
    }
}

#[tokio::test]
async fn sql_sqlite_select_literal() {
    let prepared = prepared_sql_request("sqlite::memory:", "SELECT 1 as value", vec![], false, 100);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    assert_eq!(out.url, "sql://query");

    let rows = out.result.as_array().expect("result should be an array");
    assert_eq!(rows.len(), 1);

    let row = rows[0].as_object().expect("row should be an object");
    assert_eq!(row.get("value").unwrap(), &serde_json::json!(1));
}

#[tokio::test]
async fn sql_sqlite_with_params() {
    let prepared = prepared_sql_request(
        "sqlite::memory:",
        "SELECT ? as echo",
        vec![serde_json::json!("hello")],
        false,
        100,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    let rows = out.result.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["echo"], serde_json::json!("hello"));
}

#[tokio::test]
async fn sql_max_rows_enforced() {
    // Use a recursive CTE to generate 100 rows, but limit to 5.
    let query = "\
        WITH RECURSIVE cnt(x) AS (\
            SELECT 1 UNION ALL SELECT x+1 FROM cnt WHERE x < 100\
        ) SELECT x FROM cnt";

    let prepared = prepared_sql_request("sqlite::memory:", query, vec![], false, 5);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    let rows = out.result.as_array().unwrap();
    assert_eq!(rows.len(), 5);
}

/// Test that SQLite read-only mode blocks write operations.
#[tokio::test]
async fn sql_sqlite_read_only_blocks_write() {
    // First, create a database with a table.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_string_lossy().to_string();
    let url = format!("sqlite:{db_path}");

    // Create the table (writable mode).
    let create_prepared = prepared_sql_request(
        &url,
        "CREATE TABLE test_tbl (id INTEGER PRIMARY KEY, val TEXT)",
        vec![],
        false,
        100,
    );
    execute_prepared_request_with_host_validator(&create_prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    // Now try to INSERT in read_only mode — should fail.
    let insert_prepared = prepared_sql_request(
        &url,
        "INSERT INTO test_tbl (val) VALUES ('should_fail')",
        vec![],
        true, // read_only = true
        100,
    );
    let result = execute_prepared_request_with_host_validator(&insert_prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await;

    assert!(result.is_err(), "expected INSERT to fail in read-only mode");
}

#[tokio::test]
async fn sql_invalid_query_fails() {
    let prepared = prepared_sql_request("sqlite::memory:", "INVALID SQL QUERY", vec![], false, 100);

    let result = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await;

    assert!(result.is_err(), "expected invalid SQL to produce an Err");
}
