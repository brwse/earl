#![cfg(feature = "bash")]

use std::time::Duration;

use earl::protocol::builder::{PreparedBashScript, PreparedProtocolData, PreparedRequest};
use earl::protocol::executor::execute_prepared_request_with_host_validator;
use earl::protocol::transport::ResolvedTransport;
use earl::template::schema::{CommandMode, ResultDecode, ResultTemplate};
use earl_core::Redactor;
use earl_protocol_bash::ResolvedBashSandbox;
use serde_json::Map;
use std::net::IpAddr;

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

fn prepared_bash_request(script: &str, result_template: ResultTemplate) -> PreparedRequest {
    PreparedRequest {
        key: "test.bash".to_string(),
        mode: CommandMode::Read,
        stream: false,
        allow_rules: vec![],
        transport: default_transport(),
        result_template,
        args: Map::new(),
        redactor: Redactor::default(),
        protocol_data: PreparedProtocolData::Bash(PreparedBashScript {
            script: script.to_string(),
            env: vec![],
            cwd: None,
            stdin: None,
            sandbox: default_sandbox(),
        }),
    }
}

/// Test that a simple echo command works.
#[tokio::test]
async fn bash_echo_returns_output() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let prepared = prepared_bash_request("echo hello world", result_template);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    assert_eq!(out.url, "bash://script");
    assert_eq!(out.result.as_str().unwrap().trim(), "hello world");
}

/// Test that nonzero exit codes are captured.
#[tokio::test]
async fn bash_nonzero_exit_code() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let prepared = prepared_bash_request("exit 42", result_template);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 42);
}

/// Test that stderr is captured when stdout is empty.
#[tokio::test]
async fn bash_captures_stderr() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let prepared = prepared_bash_request("echo error_msg >&2", result_template);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    assert_eq!(out.result.as_str().unwrap().trim(), "error_msg");
}

/// Test that environment variables are passed to the script.
#[tokio::test]
async fn bash_env_vars_passed() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let mut prepared = prepared_bash_request("echo $MY_VAR", result_template);
    if let PreparedProtocolData::Bash(ref mut bash) = prepared.protocol_data {
        bash.env
            .push(("MY_VAR".to_string(), "test_value_123".to_string()));
    }

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    assert_eq!(out.result.as_str().unwrap().trim(), "test_value_123");
}

/// Test that sandbox max_time_ms overrides transport timeout.
#[tokio::test]
async fn bash_sandbox_timeout_overrides_transport() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let mut prepared = prepared_bash_request("sleep 30", result_template);
    if let PreparedProtocolData::Bash(ref mut bash) = prepared.protocol_data {
        // Sandbox timeout is 500ms, transport timeout is 10s.
        // If sandbox timeout is enforced, the script will be killed quickly.
        bash.sandbox.max_time_ms = Some(500);
    }

    let start = std::time::Instant::now();
    let result = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await;

    let elapsed = start.elapsed();
    assert!(result.is_err(), "expected timeout error");
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.contains("timed out"), "unexpected error: {err}");
    assert!(
        elapsed < Duration::from_secs(5),
        "sandbox timeout should have triggered well before the transport timeout"
    );
}

/// Test that sandbox max_output_bytes is enforced.
#[tokio::test]
async fn bash_sandbox_output_limit_enforced() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let mut prepared = prepared_bash_request(
        // Generate ~10KB of output (each line is ~80 chars)
        "for i in $(seq 1 200); do echo 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'; done",
        result_template,
    );
    if let PreparedProtocolData::Bash(ref mut bash) = prepared.protocol_data {
        // Set a small output limit (1KB)
        bash.sandbox.max_output_bytes = Some(1024);
    }

    let result = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await;

    assert!(result.is_err(), "expected output limit error");
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("exceeded") || err.contains("max_response_bytes"),
        "unexpected error: {err}"
    );
}

/// Test JSON decode from bash output.
#[tokio::test]
async fn bash_json_output() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(earl::template::schema::ResultExtract::JsonPointer {
            json_pointer: "/greeting".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let prepared = prepared_bash_request(r#"echo '{"greeting": "hi"}'"#, result_template);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
    assert_eq!(out.result, serde_json::json!("hi"));
}

/// Test that sandbox max_memory_bytes is enforced.
/// macOS does not enforce RLIMIT_AS, so this test only runs on Linux.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn bash_sandbox_memory_limit_enforced() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    // Allocate 300MB; limit is 100MB.
    let mut prepared = prepared_bash_request(
        "python3 -c \"x = bytearray(300 * 1024 * 1024)\"",
        result_template,
    );
    if let PreparedProtocolData::Bash(ref mut bash) = prepared.protocol_data {
        bash.sandbox.max_memory_bytes = Some(100 * 1024 * 1024); // 100 MB
    }

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_ne!(out.status, 0, "expected non-zero exit due to memory limit");
}

/// Test that sandbox max_cpu_time_ms is enforced.
#[tokio::test]
async fn bash_sandbox_cpu_limit_enforced() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    // Tight CPU-bound loop; 1 CPU-second limit.
    let mut prepared = prepared_bash_request(
        "python3 -c \"while True: pass\"",
        result_template,
    );
    if let PreparedProtocolData::Bash(ref mut bash) = prepared.protocol_data {
        bash.sandbox.max_cpu_time_ms = Some(1_000); // 1 CPU-second
        bash.sandbox.max_time_ms = Some(5_000);     // 5s wall-clock guard
    }

    let start = std::time::Instant::now();
    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    let elapsed = start.elapsed();
    assert_ne!(out.status, 0, "expected non-zero exit due to CPU limit");
    assert!(
        elapsed < std::time::Duration::from_secs(4),
        "CPU limit should have fired before wall-clock guard"
    );
}
