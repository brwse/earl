//! Use-case tests: cookie management (Group 5).
mod common;
use common::{Response, execute, skip_if_no_chrome, spawn};
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

fn unique_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    format!("{pid}-{count}")
}

/// Test 5.1 — Server-set cookies are visible via cookie_list.
///
/// Serves a page that responds with a Set-Cookie header. After navigating to
/// it, CookieList must return an array containing the expected cookie.
#[tokio::test]
async fn server_set_cookie_visible_in_cookie_list() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /set-cookie".to_string(),
        Response::html("<html><body>cookie set</body></html>").with_cookie("token=abc123; Path=/"),
    );
    let server = spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/set-cookie"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::CookieList {
                domain: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    let cookies = result["cookies"]
        .as_array()
        .expect("result should have a 'cookies' array");

    assert!(
        cookies
            .iter()
            .any(|c| c["name"] == "token" && c["value"] == "abc123"),
        "expected cookie 'token=abc123' in cookie list; got: {result}"
    );
}

/// Test 5.2 — cookie_set makes cookie visible to the page.
///
/// After navigating to a page and setting a cookie via CookieSet, an Evaluate
/// step must report that the cookie appears in document.cookie.
#[tokio::test]
async fn cookie_set_visible_to_page() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

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
            BrowserStep::CookieSet {
                name: "theme".to_string(),
                value: "dark".to_string(),
                domain: Some("127.0.0.1".to_string()),
                path: None,
                expires: None,
                http_only: false,
                secure: false,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.cookie".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    let cookie_str = result["value"]
        .as_str()
        .expect("evaluate result should have a string 'value'");

    assert!(
        cookie_str.contains("theme=dark"),
        "expected 'theme=dark' in document.cookie; got: {cookie_str}"
    );
}

/// Test 5.3 — cookie_delete removes a cookie.
///
/// Sets a cookie, deletes it, then lists cookies to verify it is absent.
#[tokio::test]
async fn cookie_delete_removes_cookie() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

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
            BrowserStep::CookieSet {
                name: "temp".to_string(),
                value: "yes".to_string(),
                domain: Some("127.0.0.1".to_string()),
                path: None,
                expires: None,
                http_only: false,
                secure: false,
                optional: false,
            },
            BrowserStep::CookieDelete {
                name: "temp".to_string(),
                optional: false,
            },
            BrowserStep::CookieList {
                domain: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    let cookies = result["cookies"]
        .as_array()
        .expect("result should have a 'cookies' array");

    assert!(
        !cookies.iter().any(|c| c["name"] == "temp"),
        "cookie 'temp' should have been deleted; got: {result}"
    );
}

/// Test 5.4 — cookie_clear removes all cookies.
///
/// Sets two cookies, clears all cookies, then verifies the cookie list is empty.
#[tokio::test]
async fn cookie_clear_removes_all_cookies() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

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
            BrowserStep::CookieSet {
                name: "a".to_string(),
                value: "1".to_string(),
                domain: Some("127.0.0.1".to_string()),
                path: None,
                expires: None,
                http_only: false,
                secure: false,
                optional: false,
            },
            BrowserStep::CookieSet {
                name: "b".to_string(),
                value: "2".to_string(),
                domain: Some("127.0.0.1".to_string()),
                path: None,
                expires: None,
                http_only: false,
                secure: false,
                optional: false,
            },
            BrowserStep::CookieClear { optional: false },
            BrowserStep::CookieList {
                domain: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    let cookies = result["cookies"]
        .as_array()
        .expect("result should have a 'cookies' array");

    assert!(
        cookies.is_empty(),
        "cookie list should be empty after cookie_clear; got: {result}"
    );
}

/// Test 5.5 — storage_state exports and set_storage_state restores cookies across sessions.
///
/// Session A sets a cookie and writes storage state to a temp file. Session B
/// reads the state file and restores it, then verifies the cookie is present.
#[tokio::test]
async fn storage_state_round_trips_cookies_across_sessions() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body>hello</body></html>"),
    );
    let server = spawn(routes).await;

    let id = unique_id();
    // Use a relative path — validate_file_path rejects absolute paths.
    let tmp_path = format!("earl-test-state-{id}.json");

    // Session A: navigate, set cookie, export storage state to file.
    let session_a = PreparedBrowserCommand {
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
            BrowserStep::CookieSet {
                name: "auth".to_string(),
                value: "token123".to_string(),
                domain: Some("127.0.0.1".to_string()),
                path: None,
                expires: None,
                http_only: false,
                secure: false,
                optional: false,
            },
            BrowserStep::StorageState {
                path: Some(tmp_path.clone()),
                optional: false,
            },
        ],
    };

    execute(session_a).await.expect("session A should succeed");

    // Session B: navigate, restore storage state, list cookies.
    let session_b = PreparedBrowserCommand {
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
            BrowserStep::SetStorageState {
                path: tmp_path.clone(),
                optional: false,
            },
            BrowserStep::CookieList {
                domain: None,
                optional: false,
            },
        ],
    };

    let result = execute(session_b).await.expect("session B should succeed");

    std::fs::remove_file(&tmp_path).ok();

    let cookies = result["cookies"]
        .as_array()
        .expect("result should have a 'cookies' array");

    assert!(
        cookies
            .iter()
            .any(|c| c["name"] == "auth" && c["value"] == "token123"),
        "expected restored cookie 'auth=token123' in session B; got: {result}"
    );
}

/// Test 5.6 — cookie_get returns a named cookie.
///
/// Sets a cookie via CookieSet, then retrieves it by name using CookieGet and
/// verifies the returned value matches.
#[tokio::test]
async fn cookie_get_returns_named_cookie() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

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
            BrowserStep::CookieSet {
                name: "token".to_string(),
                value: "abc123".to_string(),
                domain: Some("127.0.0.1".to_string()),
                path: None,
                expires: None,
                http_only: false,
                secure: false,
                optional: false,
            },
            BrowserStep::CookieGet {
                name: "token".to_string(),
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"].as_str(),
        Some("abc123"),
        "expected cookie 'token' to have value 'abc123'; got: {result}"
    );
}
