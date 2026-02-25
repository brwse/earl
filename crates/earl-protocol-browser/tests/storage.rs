//! Use-case tests: localStorage / sessionStorage (Group 6).
mod common;
use common::{execute, skip_if_no_chrome, spawn, Response, CHROME_SERIAL};
use std::collections::HashMap;
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;

/// Test 6.1 — local_storage_set and local_storage_get round-trip.
///
/// Sets a key in localStorage and then reads it back, verifying the value
/// matches exactly what was written.
#[tokio::test]
async fn local_storage_set_and_get_round_trip() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body>hello</body></html>"),
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
            BrowserStep::LocalStorageSet {
                key: "theme".to_string(),
                value: "dark".to_string(),
                optional: false,
            },
            BrowserStep::LocalStorageGet {
                key: "theme".to_string(),
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"].as_str(),
        Some("dark"),
        "expected localStorage['theme'] == 'dark'; got: {result}"
    );
}

/// Test 6.2 — local_storage_delete removes the key.
///
/// Sets a key, deletes it, then evaluates localStorage.getItem() via JS to
/// confirm the value is null (the key no longer exists).
#[tokio::test]
async fn local_storage_delete_removes_key() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body>hello</body></html>"),
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
            BrowserStep::LocalStorageSet {
                key: "x".to_string(),
                value: "1".to_string(),
                optional: false,
            },
            BrowserStep::LocalStorageDelete {
                key: "x".to_string(),
                optional: false,
            },
            // Use Evaluate to check the key is gone; LocalStorageGet on a missing
            // key errors, and Evaluate on null also fails. Instead we check
            // localStorage.length — after setting one key and deleting it the
            // storage must be empty.
            BrowserStep::Evaluate {
                function: "() => localStorage.length".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::json!(0),
        "expected localStorage.length == 0 after delete; got: {result}"
    );
}

/// Test 6.3 — local_storage_clear wipes all keys.
///
/// Sets two keys, clears localStorage, then evaluates localStorage.length to
/// confirm no entries remain.
#[tokio::test]
async fn local_storage_clear_wipes_all_keys() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body>hello</body></html>"),
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
            BrowserStep::LocalStorageSet {
                key: "a".to_string(),
                value: "1".to_string(),
                optional: false,
            },
            BrowserStep::LocalStorageSet {
                key: "b".to_string(),
                value: "2".to_string(),
                optional: false,
            },
            BrowserStep::LocalStorageClear {
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => localStorage.length".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::json!(0),
        "expected localStorage.length == 0 after clear; got: {result}"
    );
}

/// Test 6.4 — storage_state includes localStorage entries.
///
/// Sets a localStorage key, then calls StorageState (path: None). The returned
/// `local_storage` map must include the key that was set.
///
/// The actual shape of the StorageState result (path = None) is:
/// `{"cookies": [...], "local_storage": {"key": "value", ...}}`
#[tokio::test]
async fn storage_state_includes_local_storage_entries() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = CHROME_SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body>hello</body></html>"),
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
            BrowserStep::LocalStorageSet {
                key: "pref".to_string(),
                value: "compact".to_string(),
                optional: false,
            },
            BrowserStep::StorageState {
                path: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    // storage_state returns {"cookies": [...], "local_storage": {"pref": "compact", ...}}
    let ls = result
        .get("local_storage")
        .expect("storage_state result should have a 'local_storage' field");

    assert_eq!(
        ls["pref"].as_str(),
        Some("compact"),
        "expected local_storage['pref'] == 'compact'; got: {result}"
    );
}
