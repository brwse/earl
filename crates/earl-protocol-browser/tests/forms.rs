//! Use-case tests: form automation — fill, submit, select, check.
//!
//! Each test serves a controlled local HTTP page so assertions are deterministic.
//! Chrome-dependent tests skip gracefully when Chrome is not found.

mod common;
use common::{execute, skip_if_no_chrome};

use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

/// Test 3.1 — Fill and submit a form, verify the browser navigated to /submit.
///
/// Two routes are served: GET /form with an HTML form and GET /submit with a
/// confirmation page.  The test fills the inputs, clicks submit, and verifies
/// the snapshot contains "Submitted".
#[tokio::test]
async fn fill_and_submit_form() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let form_html = r#"<html><body>
<form action="/submit" method="POST">
  <input id="email" name="email" type="text">
  <input id="name" name="name" type="text">
  <button type="submit">Submit</button>
</form>
</body></html>"#;

    let submit_html = "<html><body><p>Submitted</p></body></html>";

    let mut routes = HashMap::new();
    routes.insert(
        "GET /form".to_string(),
        common::server::Response::html(form_html),
    );
    routes.insert(
        "POST /submit".to_string(),
        common::server::Response::html(submit_html),
    );
    let server = common::server::spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 30_000,
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/form"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Fill {
                r#ref: None,
                selector: Some("#email".to_string()),
                text: "alice@example.com".to_string(),
                submit: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Fill {
                r#ref: None,
                selector: Some("#name".to_string()),
                text: "Alice".to_string(),
                submit: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Click {
                r#ref: None,
                selector: Some("button[type=submit]".to_string()),
                double_click: false,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::WaitFor {
                time: None,
                text: Some("Submitted".to_string()),
                text_gone: None,
                timeout_ms: Some(10_000),
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.body.innerText".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    let text = result["value"]
        .as_str()
        .expect("evaluate should return a string");
    assert!(
        text.contains("Submitted"),
        "body text should contain 'Submitted' (we navigated to /submit); got: {text}"
    );
}

/// Test 3.2 — Select a dropdown option and verify the change event fires.
///
/// A page with a `<select>` and a listener that writes the selected value to a
/// `<p>` is served.  After selecting "blue", `evaluate` reads the paragraph
/// text and must return `"blue"`.
#[tokio::test]
async fn select_dropdown_option() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let body = r#"<html><body>
<select id="color">
  <option value="red">Red</option>
  <option value="blue">Blue</option>
  <option value="green">Green</option>
</select>
<p id="chosen">none</p>
<script>
  document.getElementById('color').addEventListener('change', function(e) {
    document.getElementById('chosen').textContent = e.target.value;
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
            BrowserStep::SelectOption {
                r#ref: None,
                selector: Some("#color".to_string()),
                values: vec!["blue".to_string()],
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.getElementById('chosen').textContent".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("blue".to_string()),
        "evaluate should return 'blue' after selecting the option; got: {result}"
    );
}

/// Test 3.3a — Check a checkbox, then verify it is checked.
///
/// A page with a bare `<input type="checkbox">` is served.  After `check`,
/// `evaluate` must return the boolean `true`.
#[tokio::test]
async fn checkbox_can_be_checked() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(
            "<html><body><input type=\"checkbox\" id=\"tos\"></body></html>",
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
            BrowserStep::Check {
                r#ref: None,
                selector: Some("#tos".to_string()),
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.getElementById('tos').checked".to_string(),
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
        "evaluate should return true after checking the checkbox; got: {result}"
    );
}

/// Test 3.3b — Check then uncheck a checkbox, verify it ends up unchecked.
///
/// The same checkbox page is used.  After navigating, checking, and then
/// unchecking, `evaluate` must return `false`.
#[tokio::test]
async fn checked_box_can_be_unchecked() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(
            "<html><body><input type=\"checkbox\" id=\"cb\"></body></html>",
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
            BrowserStep::Check {
                r#ref: None,
                selector: Some("#cb".to_string()),
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Uncheck {
                r#ref: None,
                selector: Some("#cb".to_string()),
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.getElementById('cb').checked".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::Bool(false),
        "evaluate should return false after unchecking the checkbox; got: {result}"
    );
}

/// Test 3.4 — Optional click on absent element does not abort execution.
///
/// A simple page with no `#cookie-accept-btn` is served.  The click step uses
/// `optional: true` with a short timeout.  Execution must succeed and the
/// subsequent `evaluate` must return a string (the page title).
#[tokio::test]
async fn optional_click_on_absent_element_continues() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(
            "<html><head><title>No Cookie Banner</title></head><body><p>Hello</p></body></html>",
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
            // This element is absent; the step must be skipped silently.
            BrowserStep::Click {
                r#ref: None,
                selector: Some("#cookie-accept-btn".to_string()),
                double_click: false,
                timeout_ms: Some(1_000),
                optional: true,
            },
            BrowserStep::Evaluate {
                function: "() => document.title".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data)
        .await
        .expect("execute should succeed despite the absent element");

    assert!(
        result["value"].is_string(),
        "evaluate should return a string (the page title); got: {result}"
    );
}

/// Test 3.5 — fill with submit=true triggers form submission.
///
/// Two routes are served: GET / with a form and POST /done with a confirmation
/// page.  Setting `submit: Some(true)` on the fill step presses Enter after
/// typing, which submits the form.  The final evaluate must contain "Done".
#[tokio::test]
async fn fill_with_submit_true_submits_form() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let form_html = r#"<html><body>
<form action="/done" method="POST">
  <input id="q" name="q" type="text">
  <button type="submit">Go</button>
</form>
</body></html>"#;

    let done_html = "<html><body><p>Done</p></body></html>";

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(form_html),
    );
    routes.insert(
        "POST /done".to_string(),
        common::server::Response::html(done_html),
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
            BrowserStep::Fill {
                r#ref: None,
                selector: Some("#q".to_string()),
                text: "hello".to_string(),
                submit: Some(true),
                timeout_ms: None,
                optional: false,
            },
            // Poll until "Done" appears rather than sleeping a fixed duration.
            BrowserStep::WaitFor {
                text: Some("Done".to_string()),
                text_gone: None,
                time: None,
                timeout_ms: Some(5_000),
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.body.innerText".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    let text = result["value"]
        .as_str()
        .expect("evaluate should return a string");
    assert!(
        text.contains("Done"),
        "body text should contain 'Done' (navigated to /done via form submit with Enter); got: {text}"
    );
}

/// Test 3.6 — FillForm fills multiple fields at once and values are readable.
///
/// A page with two text inputs (id="name" and id="email") is served.
/// `FillForm` sets both fields in a single step.  An `evaluate` then reads
/// both values back; the result must be `"Alice|alice@example.com"`.
#[tokio::test]
async fn fill_form_fills_multiple_fields_at_once() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let page_html = r#"<html><body>
<form id="f">
  <input id="name" name="name" type="text">
  <input id="email" name="email" type="text">
  <button type="submit">Send</button>
</form>
</body></html>"#;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        common::server::Response::html(page_html),
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
            BrowserStep::FillForm {
                fields: vec![
                    serde_json::json!({"selector": "#name", "value": "Alice"}),
                    serde_json::json!({"selector": "#email", "value": "alice@example.com"}),
                ],
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.getElementById('name').value + '|' + document.getElementById('email').value".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");

    assert_eq!(
        result["value"],
        serde_json::Value::String("Alice|alice@example.com".to_string()),
        "evaluate should return 'Alice|alice@example.com' after FillForm; got: {result}"
    );
}
