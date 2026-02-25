//! Use-case tests: screenshot capture.
mod common;
use common::{Response, execute, skip_if_no_chrome, spawn};
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

fn unique_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{}", std::process::id(), count)
}

/// Test 2.1 — Screenshot produces a valid PNG.
///
/// Serves a simple HTML page, navigates to it, takes a viewport screenshot,
/// and verifies the returned base64 data decodes to a valid PNG image.
#[tokio::test]
async fn screenshot_produces_valid_png() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><h1>Hello</h1></body></html>"),
    );
    let server = spawn(routes).await;

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
            BrowserStep::Screenshot {
                path: None,
                r#type: None,
                full_page: false,
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    // No path given — result contains only `data`, not `path`.
    assert!(
        result["path"].is_null(),
        "result should NOT have a 'path' field when no path is given; got: {result}"
    );
    assert!(
        result["data"].is_string(),
        "result should have a 'data' string field; got: {result}"
    );

    let data_b64 = result["data"].as_str().unwrap();
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
        .expect("data should be valid base64");

    assert_eq!(
        &bytes[..4],
        &[0x89, 0x50, 0x4E, 0x47],
        "expected PNG magic bytes (\\x89PNG); got: {:?}",
        &bytes[..4]
    );
}

/// Test 2.2 — Full-page screenshot produces more data than a viewport screenshot.
///
/// Serves an HTML page containing a very tall element.  The full-page capture
/// must encode more pixels and therefore produce a longer base64 string than
/// the viewport-only capture.
#[tokio::test]
async fn full_page_screenshot_larger_than_viewport() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            r#"<html><body><div style="height:5000px;background:linear-gradient(red,blue)">tall</div></body></html>"#,
        ),
    );
    let server = spawn(routes).await;

    // Command A: viewport screenshot (full_page = false).
    let data_viewport = PreparedBrowserCommand {
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
            BrowserStep::Screenshot {
                path: None,
                r#type: None,
                full_page: false,
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result_viewport = execute(data_viewport)
        .await
        .expect("viewport screenshot should succeed");
    let data_b64_viewport = result_viewport["data"]
        .as_str()
        .expect("viewport result should have 'data' field")
        .to_string();

    // Command B: full-page screenshot (full_page = true).
    let data_full = PreparedBrowserCommand {
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
            BrowserStep::Screenshot {
                path: None,
                r#type: None,
                full_page: true,
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result_full = execute(data_full)
        .await
        .expect("full-page screenshot should succeed");
    let data_b64_full_page = result_full["data"]
        .as_str()
        .expect("full-page result should have 'data' field")
        .to_string();

    assert!(
        data_b64_full_page.len() > data_b64_viewport.len(),
        "full-page screenshot ({} chars) should be larger than viewport screenshot ({} chars)",
        data_b64_full_page.len(),
        data_b64_viewport.len()
    );
}

/// Test 2.3 — Screenshot to a specified path writes the file to disk.
///
/// Passes an explicit temp-file path to the screenshot step and verifies both
/// that the returned `path` field matches the requested path and that the file
/// exists on disk after execution.
#[tokio::test]
async fn screenshot_to_specified_path_writes_file() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><p>screenshot path test</p></body></html>"),
    );
    let server = spawn(routes).await;

    let id = unique_id();
    // Use a relative path — validate_file_path rejects absolute paths.
    let path_str = format!("earl-test-screenshot-{id}.png");

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
            BrowserStep::Screenshot {
                path: Some(path_str.clone()),
                r#type: None,
                full_page: false,
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["path"].as_str(),
        Some(path_str.as_str()),
        "result 'path' field should match the requested path; got: {}",
        result["path"]
    );
    assert!(
        std::path::Path::new(&path_str).exists(),
        "screenshot file should exist on disk at {path_str}"
    );

    std::fs::remove_file(&path_str).ok();
}
