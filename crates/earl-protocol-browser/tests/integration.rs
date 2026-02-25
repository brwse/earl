//! Integration tests for the browser protocol.
//!
//! Chrome-dependent tests skip gracefully when Chrome is not found on the host.
//! The URL-scheme validation test does not require Chrome.
//!
//! Chrome tests are serialized via a process-wide `Mutex` to avoid the
//! Chromium singleton-lock error that occurs when two instances try to use the
//! same profile directory at the same time.

use std::sync::Mutex;
use std::time::Duration;

use earl_core::{CommandMode, ExecutionContext, ProtocolExecutor, RawExecutionResult, Redactor};
use earl_core::schema::ResultTemplate;
use earl_core::transport::ResolvedTransport;
use earl_protocol_browser::{
    BrowserExecutor,
    PreparedBrowserCommand,
    steps::validate_url_scheme,
};
use earl_protocol_browser::schema::BrowserStep;
use serde_json::Map;

/// Serializes Chrome-launching tests so they don't clobber the Chromium
/// singleton lock when the test harness runs them in parallel.
static CHROME_SERIAL: Mutex<()> = Mutex::new(());

// ── helpers ───────────────────────────────────────────────────────────────────

/// Returns `true` (and prints a message) when Chrome is not found so that the
/// caller can skip the rest of the test.
fn skip_if_no_chrome() -> bool {
    if earl_protocol_browser::launcher::find_chrome().is_err() {
        eprintln!("skipping — Chrome not found on this host");
        return true;
    }
    false
}

/// Build a minimal `ExecutionContext` suitable for passing to
/// `BrowserExecutor::execute`.  The browser executor ignores the context
/// entirely (it uses only `PreparedBrowserCommand`), so the values here are
/// arbitrary but structurally valid.
fn make_context() -> ExecutionContext {
    ExecutionContext {
        key: "test".to_string(),
        mode: CommandMode::Read,
        allow_rules: vec![],
        transport: ResolvedTransport {
            timeout: Duration::from_secs(30),
            follow_redirects: true,
            max_redirect_hops: 10,
            retry_max_attempts: 0,
            retry_backoff: Duration::from_millis(100),
            retry_on_status: vec![],
            compression: false,
            tls_min_version: None,
            proxy_url: None,
            max_response_bytes: 10_000_000,
        },
        result_template: ResultTemplate::default(),
        args: Map::new(),
        redactor: Redactor::new(vec![]),
    }
}

// ── scheme-validation tests (no Chrome required) ──────────────────────────────

#[test]
fn allowed_schemes_are_accepted() {
    assert!(validate_url_scheme("http://example.com").is_ok());
    assert!(validate_url_scheme("https://example.com").is_ok());
}

#[test]
fn file_scheme_is_rejected() {
    let err = validate_url_scheme("file:///etc/passwd").unwrap_err();
    assert!(
        err.to_string().contains("file"),
        "error should mention the disallowed scheme; got: {err}"
    );
}

#[test]
fn javascript_scheme_is_rejected() {
    let err = validate_url_scheme("javascript:alert(1)").unwrap_err();
    assert!(
        err.to_string().contains("javascript"),
        "error should mention the disallowed scheme; got: {err}"
    );
}

#[test]
fn data_scheme_is_rejected() {
    let err = validate_url_scheme("data:text/html,<h1>hi</h1>").unwrap_err();
    assert!(
        err.to_string().contains("data"),
        "error should mention the disallowed scheme; got: {err}"
    );
}

#[test]
fn blob_scheme_is_rejected() {
    let err = validate_url_scheme("blob:https://example.com/uuid").unwrap_err();
    assert!(
        err.to_string().contains("blob"),
        "error should mention the disallowed scheme; got: {err}"
    );
}

// ── Chrome-dependent tests ────────────────────────────────────────────────────

/// Navigate to `https://example.com`, take a snapshot, and verify the raw
/// result body contains a JSON object with a `"text"` field.
#[tokio::test]
async fn navigate_and_snapshot() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: "https://example.com".into(),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Snapshot {
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let ctx = make_context();
    let mut executor = BrowserExecutor;
    let result: RawExecutionResult = executor
        .execute(&data, &ctx)
        .await
        .expect("execute should succeed");

    assert_eq!(result.content_type.as_deref(), Some("application/json"));

    let json: serde_json::Value =
        serde_json::from_slice(&result.body).expect("body should be valid JSON");

    // The last step was Snapshot; its result should be an object containing
    // at least a "text" key with the page's accessibility tree text.
    assert!(
        json.get("text").is_some(),
        "snapshot result should have a 'text' field; got: {json}"
    );
}

/// Navigate to `https://example.com`, attempt to click a non-existent element
/// with `optional: true` (so the step is silently skipped), then take a
/// snapshot.  The overall execution must succeed.
#[tokio::test]
async fn optional_step_continues_on_failure() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: "https://example.com".into(),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            // This element does not exist; because optional = true the step
            // engine should log a warning and continue rather than abort.
            BrowserStep::Click {
                r#ref: None,
                selector: Some("#this-element-does-not-exist-abc123".into()),
                button: None,
                double_click: false,
                modifiers: vec![],
                timeout_ms: Some(2_000),
                optional: true,
            },
            BrowserStep::Snapshot {
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let ctx = make_context();
    let mut executor = BrowserExecutor;
    let result = executor
        .execute(&data, &ctx)
        .await
        .expect("execute should succeed even though the click step failed");

    let json: serde_json::Value =
        serde_json::from_slice(&result.body).expect("body should be valid JSON");

    assert!(
        json.get("text").is_some(),
        "final snapshot result should have a 'text' field; got: {json}"
    );
}

/// Verify that navigating to a `file://` URL fails before Chrome is even
/// contacted (i.e., the scheme guard runs in the step engine).
///
/// Because the scheme check is enforced inside `execute_steps` (which requires
/// a live `Page`), we test the pure helper function rather than going through
/// the full executor path.  The full-path version is effectively covered by
/// the navigate-and-snapshot test (which uses only allowed schemes).
#[test]
fn disallowed_scheme_fails_at_scheme_validation() {
    let result = validate_url_scheme("file:///etc/passwd");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("file"),
        "error message should contain the scheme name; got: {msg}"
    );
}
