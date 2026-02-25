//! Use-case tests: keyboard and mouse coordinate interaction.
//!
//! Each test serves a controlled local HTTP page so assertions are deterministic.
//! Chrome-dependent tests skip gracefully when Chrome is not found.

mod common;
use common::{execute, skip_if_no_chrome};

use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

/// Test 11.1 — press_key fires keyboard events on the page.
///
/// A page listens for `keydown` events and writes the pressed key name to a
/// `<div>`.  After pressing "Enter", `evaluate` must return `"Enter"`.
#[tokio::test]
async fn press_key_fires_keyboard_events() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let body = r#"<html><body>
<div id="out">none</div>
<script>
document.addEventListener('keydown', function(e) {
  document.getElementById('out').textContent = e.key;
});
</script>
</body></html>"#;

    let mut routes = HashMap::new();
    routes.insert("GET /".to_string(), common::server::Response::html(body));
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
            BrowserStep::PressKey {
                key: "Enter".to_string(),
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.getElementById('out').textContent".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("Enter".to_string()),
        "evaluate should return 'Enter' after pressing the Enter key; got: {result}"
    );
}

/// Test 11.2 — mouse_wheel triggers scroll on a tall page.
///
/// A page taller than the viewport is served.  After a downward wheel event,
/// `evaluate` must return `true` for `window.scrollY > 0`.
#[tokio::test]
async fn mouse_wheel_scrolls_tall_page() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let body = r#"<html><body><div style="height:5000px">Tall content</div></body></html>"#;

    let mut routes = HashMap::new();
    routes.insert("GET /".to_string(), common::server::Response::html(body));
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
            BrowserStep::MouseWheel {
                delta_x: 0.0,
                delta_y: 500.0,
                optional: false,
            },
            // MouseWheel dispatches synchronously; evaluate scroll position directly.
            BrowserStep::Evaluate {
                function: "() => window.scrollY > 0".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::Bool(true),
        "evaluate should return true (scrollY > 0) after mouse wheel; got: {result}"
    );
}
