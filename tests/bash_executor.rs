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
        allow_private_ips: false,
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

fn prepared_bash_request_with_sandbox(
    script: &str,
    result_template: ResultTemplate,
    sandbox: ResolvedBashSandbox,
) -> PreparedRequest {
    PreparedRequest {
        key: "test.bash".to_string(),
        mode: CommandMode::Read,
        stream: false,
        allow_rules: vec![],
        allow_private_ips: false,
        transport: default_transport(),
        result_template,
        args: Map::new(),
        redactor: Redactor::default(),
        protocol_data: PreparedProtocolData::Bash(PreparedBashScript {
            script: script.to_string(),
            env: vec![],
            cwd: None,
            stdin: None,
            sandbox,
        }),
    }
}

fn prepared_bash_request_with_env(
    script: &str,
    result_template: ResultTemplate,
    env: Vec<(String, String)>,
) -> PreparedRequest {
    PreparedRequest {
        key: "test.bash".to_string(),
        mode: CommandMode::Read,
        stream: false,
        allow_rules: vec![],
        allow_private_ips: false,
        transport: default_transport(),
        result_template,
        args: Map::new(),
        redactor: Redactor::default(),
        protocol_data: PreparedProtocolData::Bash(PreparedBashScript {
            script: script.to_string(),
            env,
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

    assert_eq!(out.result.as_str().unwrap().trim(), "hello world");
}

/// Test that nonzero exit codes are captured.
#[tokio::test]
async fn bash_nonzero_exit_code_is_captured() {
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

    assert_eq!(out.result.as_str().unwrap().trim(), "error_msg");
}

/// Test that environment variables are accessible inside the script.
#[tokio::test]
async fn bash_env_var_is_accessible_in_script() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let prepared = prepared_bash_request_with_env(
        "echo $MY_VAR",
        result_template,
        vec![("MY_VAR".to_string(), "test_value_123".to_string())],
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.result.as_str().unwrap().trim(), "test_value_123");
}

/// Test that shell metacharacters in an env var value are not interpreted as shell code.
/// This verifies the safe arg-passing pattern: env vars referenced as "$VAR" treat their
/// value as a literal string, so `. ; echo injected` does not execute a second command.
#[tokio::test]
async fn bash_env_var_metacharacters_are_not_interpreted() {
    let result_template = ResultTemplate {
        decode: ResultDecode::Text,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let prepared = prepared_bash_request_with_env(
        r#"echo "$EARL_PATH""#,
        result_template,
        vec![("EARL_PATH".to_string(), ". ; echo injected".to_string())],
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    let output = out.result.as_str().unwrap().trim();
    // The value should be echoed literally, not interpreted as two commands.
    assert_eq!(output, ". ; echo injected");
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

    // Sandbox timeout is 500ms, transport timeout is 10s.
    // If sandbox timeout is enforced, the script will be killed quickly.
    let prepared = prepared_bash_request_with_sandbox(
        "sleep 30",
        result_template,
        ResolvedBashSandbox {
            max_time_ms: Some(500),
            ..default_sandbox()
        },
    );

    let start = std::time::Instant::now();
    let result = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await;

    let elapsed = start.elapsed();
    assert!(result.is_err(), "expected timeout error");
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

    // Generate ~10KB of output (each line is ~80 chars); limit is 1KB.
    let prepared = prepared_bash_request_with_sandbox(
        "for i in $(seq 1 200); do echo 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'; done",
        result_template,
        ResolvedBashSandbox {
            max_output_bytes: Some(1024),
            ..default_sandbox()
        },
    );

    let result = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await;

    assert!(result.is_err(), "expected output limit error");
}

/// Test JSON decode from bash output.
#[tokio::test]
async fn bash_json_output_is_decoded_and_extracted() {
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
    let prepared = prepared_bash_request_with_sandbox(
        "python3 -c \"x = bytearray(300 * 1024 * 1024)\"",
        result_template,
        ResolvedBashSandbox {
            max_memory_bytes: Some(100 * 1024 * 1024), // 100 MB
            ..default_sandbox()
        },
    );

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

    // Tight CPU-bound loop; 1 CPU-second limit with 5s wall-clock guard.
    let prepared = prepared_bash_request_with_sandbox(
        "python3 -c \"while True: pass\"",
        result_template,
        ResolvedBashSandbox {
            max_cpu_time_ms: Some(1_000), // 1 CPU-second
            max_time_ms: Some(5_000),     // 5s wall-clock guard
            ..default_sandbox()
        },
    );

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
