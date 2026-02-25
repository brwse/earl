//! Use-case tests: web scraping — extracting content from pages.
//!
//! Each test serves a controlled local HTTP page so assertions are deterministic.
//! Chrome-dependent tests skip gracefully when Chrome is not found.

mod common;
use common::{execute, skip_if_no_chrome, CHROME_SERIAL};

use std::collections::HashMap;
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;

/// Test 1.1 — Extract static text via evaluate.
///
/// Serve a page with a static `<h1>` heading and use `evaluate` to read its
/// text content.
#[tokio::test]
async fn extract_static_text_via_evaluate() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html("<h1>Product: Acme Widget</h1>"),
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
                function: "() => document.querySelector('h1').textContent".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("Product: Acme Widget".to_string()),
        "evaluate should return the h1 text content; got: {result}"
    );
}

/// Test 1.2 — Extract dynamically-rendered text using wait_for.
///
/// The page uses `setTimeout` to inject a `<p>` element after 300ms.  The
/// `wait_for` step must observe the text before `evaluate` reads it.
#[tokio::test]
async fn extract_dynamic_text_with_wait_for() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let body = r#"<html><body>
<script>
setTimeout(function() {
  var p = document.createElement('p');
  p.id = 'loaded';
  p.textContent = 'Data loaded';
  document.body.appendChild(p);
}, 300);
</script>
</body></html>"#;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(body),
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
            BrowserStep::WaitFor {
                time: None,
                text: Some("Data loaded".to_string()),
                text_gone: None,
                timeout_ms: 5_000,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.getElementById('loaded').textContent".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("Data loaded".to_string()),
        "evaluate should return the dynamically-injected text; got: {result}"
    );
}

/// Test 1.3 — wait_for times out when content never appears.
///
/// A blank page is served and `wait_for` looks for text that is never added.
/// The execution must return an error.
#[tokio::test]
async fn wait_for_times_out_when_content_absent() {
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
            BrowserStep::WaitFor {
                time: None,
                text: Some("content that will never appear".to_string()),
                text_gone: None,
                timeout_ms: 500,
                optional: false,
            },
        ],
    };

    let err = execute(data).await.expect_err("execute should fail with a timeout error");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("wait_for")
            || msg.to_lowercase().contains("timed out")
            || msg.to_lowercase().contains("timeout"),
        "error should mention a timeout or wait_for; got: {msg}"
    );
}

/// Test 1.4 — Navigate to a second page; the last snapshot reflects the second page.
///
/// Two routes are served.  After navigating to each in sequence, a `Snapshot`
/// is taken and its `"text"` field must contain content from the second page.
#[tokio::test]
async fn multi_navigate_snapshot_reflects_last_page() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /page1".to_string(),
        common::server::Response::html(
            "<html><head><title>Page One</title></head><body><p>Page One content</p></body></html>",
        ),
    );
    routes.insert(
        "GET /page2".to_string(),
        common::server::Response::html(
            "<html><head><title>Page Two</title></head><body><p>Page Two content</p></body></html>",
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
                url: server.url("/page1"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Navigate {
                url: server.url("/page2"),
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

    let result = execute(data).await.expect("execute should succeed");

    let text = result["text"]
        .as_str()
        .expect("snapshot result should have a 'text' string field");
    assert!(
        text.contains("Page Two"),
        "snapshot text should contain 'Page Two' (the second page); got: {text}"
    );
}
