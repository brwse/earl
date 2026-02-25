//! Use-case tests: PDF save.
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

/// Test 8.1 — pdf_save writes a valid PDF to disk.
///
/// Serves an HTML invoice page, requests a PDF at an explicit temp path, and
/// verifies that the file exists and starts with the PDF magic bytes `%PDF`.
#[tokio::test]
async fn pdf_save_writes_valid_pdf_to_disk() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><h1>Invoice #001</h1><p>Total: $42.00</p></body></html>"),
    );
    let server = spawn(routes).await;

    let id = unique_id();
    // Use a relative path — validate_file_path rejects absolute paths.
    let path_str = format!("earl-test-invoice-{id}.pdf");

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
            BrowserStep::PdfSave {
                path: Some(path_str.clone()),
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
        "PDF file should exist on disk at {path_str}"
    );

    // Verify the PDF magic bytes: %PDF = [0x25, 0x50, 0x44, 0x46]
    let bytes = std::fs::read(&path_str).expect("should be able to read the PDF file");
    assert!(
        bytes.len() >= 4,
        "PDF file should contain at least 4 bytes; got {} bytes",
        bytes.len()
    );
    assert_eq!(
        &bytes[..4],
        &[0x25, 0x50, 0x44, 0x46],
        "expected PDF magic bytes (%PDF); got: {:?}",
        &bytes[..4]
    );

    std::fs::remove_file(&path_str).ok();
}

/// Test 8.2 — pdf_save with no path returns base64 data without writing a file.
///
/// Omits the `path` field.  The executor must return `{"data": "<base64>",
/// "size": N}` without touching the file system.  The base64 data must decode
/// to valid PDF bytes (magic header `%PDF`).
#[tokio::test]
async fn pdf_save_no_path_returns_base64_data() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><p>base64 PDF test</p></body></html>"),
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
            BrowserStep::PdfSave {
                path: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    // No path given — result must have `data`, not `path`.
    assert!(
        result["path"].is_null(),
        "result should NOT have a 'path' field when no path is given; got: {result}"
    );

    let data_b64 = result["data"]
        .as_str()
        .expect("result should have a non-null 'data' field");
    assert!(!data_b64.is_empty(), "base64 data should be non-empty");

    let size = result["size"]
        .as_u64()
        .expect("result should have a numeric 'size' field");
    assert!(size > 0, "size should be greater than 0");

    // Decode and verify PDF magic bytes: %PDF = [0x25, 0x50, 0x44, 0x46]
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data_b64)
        .expect("data should be valid base64");
    assert!(
        bytes.len() >= 4,
        "decoded PDF should have at least 4 bytes; got {}",
        bytes.len()
    );
    assert_eq!(
        &bytes[..4],
        &[0x25, 0x50, 0x44, 0x46],
        "expected PDF magic bytes (%PDF); got: {:?}",
        &bytes[..4]
    );
}
