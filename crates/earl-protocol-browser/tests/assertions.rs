//! Use-case tests: multi-step assertion steps (Group 9).
mod common;
use common::{Response, execute, skip_if_no_chrome, spawn};
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

/// Test 9.1 — verify_text_visible passes when text is present.
#[tokio::test]
async fn verify_text_visible_passes_when_text_present() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><p>Order confirmed</p></body></html>"),
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
            BrowserStep::VerifyTextVisible {
                text: "Order confirmed".to_string(),
                optional: false,
            },
        ],
    };

    execute(data).await.expect(
        "VerifyTextVisible should succeed when the text 'Order confirmed' is present in the DOM",
    );
}

/// Test 9.2 — verify_text_visible fails when text is absent.
#[tokio::test]
async fn verify_text_visible_fails_when_text_absent() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><p>Nothing here</p></body></html>"),
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
            BrowserStep::VerifyTextVisible {
                text: "Order confirmed".to_string(),
                optional: false,
            },
        ],
    };

    let result = execute(data).await;
    assert!(
        result.is_err(),
        "VerifyTextVisible should fail with an error when 'Order confirmed' is not present; got Ok: {:?}",
        result.ok()
    );
}

/// Test 9.3 — verify_element_visible passes when element exists.
#[tokio::test]
async fn verify_element_visible_passes_when_element_exists() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            r#"<html><body><button role="button" aria-label="Submit">Submit</button></body></html>"#,
        ),
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
            BrowserStep::VerifyElementVisible {
                role: Some("button".to_string()),
                accessible_name: Some("Submit".to_string()),
                optional: false,
            },
        ],
    };

    execute(data).await.expect(
        "VerifyElementVisible should succeed when a button with aria-label 'Submit' is present",
    );
}

/// Test 9.4 — verify_element_visible fails when the element is absent (Issue I4).
#[tokio::test]
async fn verify_element_visible_fails_when_element_absent() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body><p>No ghost here</p></body></html>"),
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
            BrowserStep::VerifyElementVisible {
                role: Some("button".to_string()),
                accessible_name: Some("ghost".to_string()),
                optional: false,
            },
        ],
    };

    let err = execute(data)
        .await
        .expect_err("should fail when element is absent");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("ghost") || msg.contains("not found") || msg.contains("visible"),
        "error message should mention 'ghost', 'not found', or 'visible'; got: {err}"
    );
}

/// Test 9.5 — verify_list_visible passes when all items are present (Issue I5).
#[tokio::test]
async fn verify_list_visible_passes_when_all_items_present() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            "<html><body><ul><li>Apple</li><li>Banana</li><li>Cherry</li></ul></body></html>",
        ),
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
            BrowserStep::VerifyListVisible {
                r#ref: None,
                items: vec![
                    "Apple".to_string(),
                    "Banana".to_string(),
                    "Cherry".to_string(),
                ],
                optional: false,
            },
        ],
    };

    execute(data)
        .await
        .expect("VerifyListVisible should succeed when all items are present in the DOM");
}

/// Test 9.6 — verify_list_visible fails when an item is missing (Issue I5).
#[tokio::test]
async fn verify_list_visible_fails_when_item_missing() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            "<html><body><ul><li>Apple</li><li>Banana</li><li>Cherry</li></ul></body></html>",
        ),
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
            BrowserStep::VerifyListVisible {
                r#ref: None,
                items: vec![
                    "Apple".to_string(),
                    "Banana".to_string(),
                    "Durian".to_string(),
                ],
                optional: false,
            },
        ],
    };

    execute(data)
        .await
        .expect_err("should fail when item 'Durian' is missing from the page");
}

/// Test 9.7 — verify_value matches the value attribute of a focused input field (Issue I5).
#[tokio::test]
async fn verify_value_matches_input_field_value() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(r#"<html><body><input id="price" value="42.00"></body></html>"#),
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
            // Focus the input so that step_verify_value picks it up via document.activeElement.
            BrowserStep::Evaluate {
                function: "() => { document.getElementById('price').focus(); return true; }"
                    .to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::VerifyValue {
                r#ref: None,
                value: "42.00".to_string(),
                optional: false,
            },
        ],
    };

    let result = execute(data)
        .await
        .expect("VerifyValue should succeed when the focused input has value '42.00'");
    assert_eq!(
        result["ok"],
        serde_json::Value::Bool(true),
        "VerifyValue result should be {{\"ok\": true}}; got: {result}"
    );
}
