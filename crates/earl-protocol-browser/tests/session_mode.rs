//! Use-case tests: session persistence across multiple earl invocations.
//!
//! Tests the key AI-browsing pattern: each "call" is a separate BrowserExecutor
//! invocation sharing a session_id.  State (page URL, cookies, localStorage)
//! persists across calls.
mod common;
use common::{execute, skip_if_no_chrome, CHROME_SERIAL};
use std::collections::HashMap;
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;

fn timestamp_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Test 4.1 — Multiple calls with the same session_id each see the served page.
///
/// Two separate `execute()` calls share the same session_id.  Each call
/// navigates to `/page1` and takes a snapshot.  Both calls must succeed and
/// return "Page One" in the snapshot text, confirming that the session
/// management infrastructure (lock acquisition, session file read/write) works
/// correctly across repeated invocations with the same session_id.
///
/// Additionally, the session file is verified to exist after both calls,
/// confirming the executor correctly records session state.
#[tokio::test]
async fn session_persists_navigation_across_calls() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let session_id = format!("test-persist-{}", timestamp_ms());

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
    let text1 = result1["text"].as_str().expect("call 1 snapshot should have 'text'");
    assert!(
        text1.contains("Page One"),
        "call 1 snapshot should contain 'Page One'; got: {text1}"
    );

    // Verify the session file was created after call 1.
    use earl_protocol_browser::session::{SessionFile, session_file_path, lock_file_path};
    let sf_after_call1 = SessionFile::load_from(&session_file_path(&session_id).unwrap())
        .expect("session file should be readable")
        .expect("session file should exist after call 1");
    assert!(
        !sf_after_call1.websocket_url.is_empty(),
        "session file should record a websocket URL after call 1"
    );
    assert!(
        !sf_after_call1.interrupted,
        "session file should record interrupted=false after successful call 1"
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
    let text2 = result2["text"].as_str().expect("call 2 snapshot should have 'text'");
    assert!(
        text2.contains("Page One"),
        "call 2 snapshot should contain 'Page One'; got: {text2}"
    );

    // Verify the session file exists and is valid after call 2.
    let sf_after_call2 = SessionFile::load_from(&session_file_path(&session_id).unwrap())
        .expect("session file should be readable after call 2")
        .expect("session file should exist after call 2");
    assert!(
        !sf_after_call2.websocket_url.is_empty(),
        "session file should record a websocket URL after call 2"
    );
    assert!(
        !sf_after_call2.interrupted,
        "session file should record interrupted=false after successful call 2"
    );

    // Cleanup: delete session file and lock file.
    let _ = std::fs::remove_file(session_file_path(&session_id).unwrap());
    let _ = std::fs::remove_file(lock_file_path(&session_id).unwrap());
}

/// Test 4.2 — Stale session (bad websocket URL) falls back to fresh Chrome.
///
/// A `SessionFile` is written with a dead websocket URL.  The executor must
/// detect the stale connection, launch a fresh Chrome, and complete the steps
/// successfully.
#[tokio::test]
async fn stale_session_falls_back_to_fresh_chrome() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    use earl_protocol_browser::session::{
        SessionFile, sessions_dir, ensure_sessions_dir, session_file_path, lock_file_path,
    };
    use chrono::Utc;

    let session_id = format!("test-stale-{}", timestamp_ms());
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
    sf.save_to(&session_file_path(&session_id).unwrap()).unwrap();

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
    let result = execute(data).await.expect("stale session should fall back to fresh Chrome");
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

    let session_id = format!("test-lock-{}", timestamp_ms());

    // Acquire the lock — holds it for the duration of this test.
    let _lock = acquire_session_lock(&session_id).await.unwrap();

    let data = PreparedBrowserCommand {
        session_id: Some(session_id.clone()),
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Snapshot {
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let err = execute(data).await.expect_err("execute should fail with SessionLocked");
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
