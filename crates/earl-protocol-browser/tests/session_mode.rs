//! Use-case tests: session persistence across multiple earl invocations.
//!
//! Tests the key AI-browsing pattern: each "call" is a separate BrowserExecutor
//! invocation sharing a session_id.  State (page URL, cookies, localStorage)
//! persists across calls.
mod common;
use common::{execute, skip_if_no_chrome};
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

/// Return a string that is unique within this process across concurrent calls.
///
/// Combines the process ID with a monotonically increasing atomic counter so
/// that two concurrent tests never generate the same session ID, even when they
/// happen to run within the same millisecond.
fn unique_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("{pid}-{count}")
}

/// Test 4.1a — Call 1 navigates to /page1 and the snapshot contains "Page One".
#[tokio::test]
async fn session_call_1_succeeds() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let session_id = format!("test-persist-{}", unique_id());

    let mut routes = HashMap::new();
    routes.insert(
        "GET /page1".to_string(),
        common::server::Response::html(
            "<html><head><title>Page One</title></head><body><p>Page One is here</p></body></html>",
        ),
    );
    let server = common::server::spawn(routes).await;

    let call1 = PreparedBrowserCommand {
        session_id: Some(session_id.clone()),
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
            BrowserStep::Snapshot {
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result1 = execute(call1).await.expect("call 1 should succeed");
    let text1 = result1["text"]
        .as_str()
        .expect("call 1 snapshot should have 'text'");
    assert!(
        text1.contains("Page One"),
        "call 1 snapshot should contain 'Page One'; got: {text1}"
    );

    // Cleanup: session files use unique IDs so stale files are harmless, but
    // clean up eagerly to avoid accumulation on long-running CI machines.
    use earl_protocol_browser::session::{lock_file_path, session_file_path};
    let _ = std::fs::remove_file(session_file_path(&session_id).unwrap());
    let _ = std::fs::remove_file(lock_file_path(&session_id).unwrap());
}

/// Test 4.1b — Call 1 then call 2 both succeed with the same session_id,
/// and both snapshots contain "Page One".
///
/// Verifies that the session management infrastructure (lock acquisition,
/// session file read/write) works correctly across repeated invocations with
/// the same session_id.
#[tokio::test]
async fn session_call_2_reuses_same_session() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let session_id = format!("test-persist-{}", unique_id());

    let mut routes = HashMap::new();
    routes.insert(
        "GET /page1".to_string(),
        common::server::Response::html(
            "<html><head><title>Page One</title></head><body><p>Page One is here</p></body></html>",
        ),
    );
    let server = common::server::spawn(routes).await;

    // Call 1: navigate to /page1 and take a snapshot.
    let call1 = PreparedBrowserCommand {
        session_id: Some(session_id.clone()),
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
            BrowserStep::Snapshot {
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result1 = execute(call1).await.expect("call 1 should succeed");
    let text1 = result1["text"]
        .as_str()
        .expect("call 1 snapshot should have 'text'");
    assert!(
        text1.contains("Page One"),
        "call 1 snapshot should contain 'Page One'; got: {text1}"
    );

    // Call 2: navigate to /page1 again with the same session_id.
    // The session management infrastructure (lock, session file) must handle
    // this correctly — even if the underlying Chrome instance is recycled.
    let call2 = PreparedBrowserCommand {
        session_id: Some(session_id.clone()),
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
            BrowserStep::Snapshot {
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result2 = execute(call2).await.expect("call 2 should succeed");
    let text2 = result2["text"]
        .as_str()
        .expect("call 2 snapshot should have 'text'");
    assert!(
        text2.contains("Page One"),
        "call 2 snapshot should contain 'Page One'; got: {text2}"
    );

    // Cleanup.
    use earl_protocol_browser::session::{lock_file_path, session_file_path};
    let _ = std::fs::remove_file(session_file_path(&session_id).unwrap());
    let _ = std::fs::remove_file(lock_file_path(&session_id).unwrap());
}

/// Test 4.2 — Stale session (bad websocket URL) falls back to fresh Chrome.
///
/// A `SessionFile` is written with a dead websocket URL.  The executor must
/// detect the stale connection, launch a fresh Chrome, and complete the steps
/// successfully.
///
/// Note: internal types (`SessionFile`, `session_file_path`, etc.) are used
/// here deliberately to set up the stale-session precondition.  There is no
/// public API for injecting a fake session file, so white-box setup is the
/// only viable approach for this test.
#[tokio::test]
async fn stale_session_falls_back_to_fresh_chrome() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    use chrono::Utc;
    use earl_protocol_browser::session::{
        SessionFile, ensure_sessions_dir, lock_file_path, session_file_path, sessions_dir,
    };

    let session_id = format!("test-stale-{}", unique_id());
    let dir = sessions_dir().unwrap();
    ensure_sessions_dir(&dir).unwrap();

    // Write a stale session file with a dead websocket URL.
    let sf = SessionFile {
        pid: 0,
        websocket_url: "ws://127.0.0.1:1/devtools/browser/fake".to_string(),
        target_id: "fake".to_string(),
        started_at: Utc::now(),
        last_used_at: Utc::now(),
        interrupted: false,
    };
    sf.save_to(&session_file_path(&session_id).unwrap())
        .unwrap();

    let mut routes = HashMap::new();
    routes.insert(
        "GET /stale-check".to_string(),
        common::server::Response::html(
            "<html><head><title>Fresh Chrome</title></head><body><p>Fresh Chrome loaded</p></body></html>",
        ),
    );
    let server = common::server::spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: Some(session_id.clone()),
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/stale-check"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            // Evaluate document.title which is populated immediately after navigation.
            BrowserStep::Evaluate {
                function: "() => document.title".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    // Execution must succeed — fresh Chrome was launched after detecting stale session.
    let result = execute(data)
        .await
        .expect("stale session should fall back to fresh Chrome");
    let title = result["value"]
        .as_str()
        .expect("evaluate should return the page title as 'value'");
    assert_eq!(
        title, "Fresh Chrome",
        "title should be 'Fresh Chrome' confirming navigation succeeded; got: {title}"
    );

    // Cleanup.
    let _ = std::fs::remove_file(session_file_path(&session_id).unwrap());
    let _ = std::fs::remove_file(lock_file_path(&session_id).unwrap());
}

/// Test 4.3 — Concurrent lock on same session_id returns SessionLocked error.
///
/// The lock is acquired manually before calling `execute()`.  The executor must
/// return an error immediately without trying to launch Chrome.
///
/// This test does NOT require Chrome.
#[tokio::test]
async fn concurrent_lock_returns_session_locked_error() {
    use earl_protocol_browser::session::{acquire_session_lock, lock_file_path};

    let session_id = format!("test-lock-{}", unique_id());

    // Acquire the lock — holds it for the duration of this test.
    let _lock = acquire_session_lock(&session_id).await.unwrap();

    let data = PreparedBrowserCommand {
        session_id: Some(session_id.clone()),
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![BrowserStep::Snapshot {
            timeout_ms: None,
            optional: false,
        }],
    };

    let err = execute(data)
        .await
        .expect_err("execute should fail with SessionLocked");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("locked") || msg.contains("SessionLocked"),
        "error should mention 'locked' or 'SessionLocked'; got: {msg}"
    );

    // Drop the lock before cleanup.
    drop(_lock);

    // Cleanup: delete lock file.
    let _ = std::fs::remove_file(lock_file_path(&session_id).unwrap());
}
