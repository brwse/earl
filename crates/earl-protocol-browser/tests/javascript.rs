//! Use-case tests: JavaScript execution — evaluate and run_code steps.
//!
//! Each test serves a controlled local HTTP page so assertions are deterministic.
//! Chrome-dependent tests skip gracefully when Chrome is not found.

mod common;
use common::{execute, skip_if_no_chrome, CHROME_SERIAL};

use std::collections::HashMap;
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;

/// Test 10.1 — evaluate returns a JS expression result.
///
/// A simple arithmetic expression `1 + 1` must evaluate to the JSON number `2`.
#[tokio::test]
async fn evaluate_returns_expression_result() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html("<html><body></body></html>"),
    );
    let server = common::server::spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => 1 + 1".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::Number(serde_json::Number::from(2)),
        "evaluate should return 2 as a JSON number; got: {result}"
    );
}

/// Test 10.2 — evaluate reads the DOM title.
///
/// A page is served with a known `<title>` element.  The `evaluate` step must
/// return the page title as a JSON string.
#[tokio::test]
async fn evaluate_reads_dom_title() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(
            "<html><head><title>My Test Title</title></head><body></body></html>",
        ),
    );
    let server = common::server::spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.title".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("My Test Title".to_string()),
        "evaluate should return the page title; got: {result}"
    );
}

/// Test 10.3 — run_code executes multi-statement code.
///
/// A multi-statement code block computes `40 + 2` and returns `42`.  The
/// result must be the JSON number `42`.
#[tokio::test]
async fn run_code_executes_multi_statement() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html("<html><body></body></html>"),
    );
    let server = common::server::spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::RunCode {
                code: "const x = 40; const y = 2; return x + y;".to_string(),
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::Number(serde_json::Number::from(42)),
        "run_code should return 42 as a JSON number; got: {result}"
    );
}

/// Test 10.4 — run_code mutations are visible to a subsequent evaluate.
///
/// `run_code` sets `document.title` to a known string.  A subsequent
/// `evaluate` step must observe the mutation.
#[tokio::test]
async fn run_code_mutation_visible_to_evaluate() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(
            "<html><head><title>original</title></head><body></body></html>",
        ),
    );
    let server = common::server::spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::RunCode {
                code: "document.title = 'injected'; return document.title;".to_string(),
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.title".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("injected".to_string()),
        "evaluate should see the title mutated by run_code; got: {result}"
    );
}
