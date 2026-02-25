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
        max_memory_bytes: None,
        max_cpu_time_ms: None,
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

async fn collect_output(mut rx: mpsc::Receiver<StreamChunk>) -> String {
    let mut out = String::new();
    while let Some(chunk) = rx.recv().await {
        out.push_str(&String::from_utf8(chunk.data).unwrap());
    }
    out
}

async fn collect_chunks(mut rx: mpsc::Receiver<StreamChunk>) -> Vec<String> {
    let mut chunks = vec![];
    while let Some(chunk) = rx.recv().await {
        chunks.push(String::from_utf8(chunk.data).unwrap().trim_end().to_string());
    }
    chunks
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

    let (tx, rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    let combined = collect_output(rx).await;
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

    let (tx, _rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let meta = executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    assert_eq!(meta.status, 42);
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
            max_memory_bytes: None,
            max_cpu_time_ms: None,
        },
    };

    let (tx, _rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let result = executor.execute_stream(&script, &context, tx).await;

    assert!(result.is_err(), "expected output limit error");
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

    let (tx, rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    let lines = collect_chunks(rx).await;
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

    let (tx, rx) = mpsc::channel::<StreamChunk>(16);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    executor
        .execute_stream(&script, &context, tx)
        .await
        .unwrap();

    let combined = collect_output(rx).await;
    assert!(
        combined.contains("streamed_value"),
        "expected env var value in: {combined}"
    );
}

#[tokio::test]
async fn bash_streaming_stops_when_receiver_drops() {
    // Start a long-running script that would run for a while.
    let script = PreparedBashScript {
        script: "for i in $(seq 1 1000); do echo line$i; sleep 0.01; done".to_string(),
        env: vec![],
        cwd: None,
        stdin: None,
        sandbox: default_sandbox(),
    };

    let (tx, mut rx) = mpsc::channel::<StreamChunk>(4);
    let context = default_context();

    let mut executor = BashStreamExecutor;
    let handle = tokio::spawn(async move { executor.execute_stream(&script, &context, tx).await });

    // Read a few chunks then drop the receiver.
    rx.recv().await;
    rx.recv().await;
    rx.recv().await;
    drop(rx);

    // The executor should finish without error (or finish within a reasonable time).
    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(
        result.is_ok(),
        "executor should finish quickly after receiver is dropped"
    );
    // The result can be Ok or Err depending on timing — the important thing is it stops.
}
