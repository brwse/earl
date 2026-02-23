#![cfg(feature = "bash")]

use std::time::Duration;

use earl_core::schema::{CommandMode, ResultTemplate};
use earl_core::transport::ResolvedTransport;
use earl_core::{ExecutionContext, Redactor, StreamChunk, StreamingProtocolExecutor};
use earl_protocol_bash::{BashStreamExecutor, PreparedBashScript, ResolvedBashSandbox};
use serde_json::Map;
use tokio::sync::mpsc;

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

fn default_sandbox() -> ResolvedBashSandbox {
    ResolvedBashSandbox {
        network: false,
        writable_paths: vec![],
        max_time_ms: None,
        max_output_bytes: None,
    }
}

fn default_context() -> ExecutionContext {
    ExecutionContext {
        key: "test".to_string(),
        mode: CommandMode::Read,
        allow_rules: vec![],
        transport: default_transport(),
        result_template: ResultTemplate::default(),
        args: Map::new(),
        redactor: Redactor::new(vec![]),
    }
}

#[tokio::test]
async fn bash_streaming_sends_output_as_chunks() {
    let script = PreparedBashScript {
        script: "echo line1; echo line2; echo line3".to_string(),
        env: vec![],
        cwd: None,
        stdin: None,
        sandbox: default_sandbox(),
    };

    let (tx, mut rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let meta = executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    assert_eq!(meta.status, 0);
    assert_eq!(meta.url, "bash://script");

    let mut chunks = vec![];
    while let Some(chunk) = rx.recv().await {
        chunks.push(String::from_utf8(chunk.data).unwrap());
    }
    let combined: String = chunks.concat();
    assert!(combined.contains("line1"), "missing line1 in: {combined}");
    assert!(combined.contains("line2"), "missing line2 in: {combined}");
    assert!(combined.contains("line3"), "missing line3 in: {combined}");
}

#[tokio::test]
async fn bash_streaming_captures_exit_code() {
    let script = PreparedBashScript {
        script: "echo done; exit 42".to_string(),
        env: vec![],
        cwd: None,
        stdin: None,
        sandbox: default_sandbox(),
    };

    let (tx, mut rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let meta = executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    assert_eq!(meta.status, 42);

    // Drain the channel so we confirm output was still sent.
    let mut chunks = vec![];
    while let Some(chunk) = rx.recv().await {
        chunks.push(String::from_utf8(chunk.data).unwrap());
    }
    let combined: String = chunks.concat();
    assert!(combined.contains("done"), "missing 'done' in: {combined}");
}

#[tokio::test]
async fn bash_streaming_respects_output_limit() {
    let script = PreparedBashScript {
        script: "for i in $(seq 1 200); do echo 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'; done".to_string(),
        env: vec![],
        cwd: None,
        stdin: None,
        sandbox: ResolvedBashSandbox {
            network: false,
            writable_paths: vec![],
            max_time_ms: None,
            max_output_bytes: Some(1024),
        },
    };

    let (tx, _rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let result = executor.execute_stream(&script, &context, tx).await;

    assert!(result.is_err(), "expected output limit error");
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("exceeded"), "unexpected error message: {err}");
}

#[tokio::test]
async fn bash_streaming_each_line_is_separate_chunk() {
    let script = PreparedBashScript {
        script: "echo alpha; echo beta; echo gamma".to_string(),
        env: vec![],
        cwd: None,
        stdin: None,
        sandbox: default_sandbox(),
    };

    let (tx, mut rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let meta = executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    assert_eq!(meta.status, 0);

    let mut lines = vec![];
    while let Some(chunk) = rx.recv().await {
        let text = String::from_utf8(chunk.data).unwrap();
        lines.push(text.trim_end().to_string());
    }

    assert_eq!(lines, vec!["alpha", "beta", "gamma"]);
}

#[tokio::test]
async fn bash_streaming_env_vars_passed() {
    let script = PreparedBashScript {
        script: "echo $MY_VAR".to_string(),
        env: vec![("MY_VAR".to_string(), "streamed_value".to_string())],
        cwd: None,
        stdin: None,
        sandbox: default_sandbox(),
    };

    let (tx, mut rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let meta = executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    assert_eq!(meta.status, 0);

    let mut chunks = vec![];
    while let Some(chunk) = rx.recv().await {
        chunks.push(String::from_utf8(chunk.data).unwrap());
    }
    let combined: String = chunks.concat();
    assert!(
        combined.contains("streamed_value"),
        "expected env var value in: {combined}"
    );
}
