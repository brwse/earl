//! Use-case tests: dialog handling (Group 7).
//!
//! Dialogs are triggered via `window.onload` + `setTimeout(..., 100)` so
//! that the Navigate step completes before the dialog fires.  HandleDialog
//! subscribes to `Page.javascriptDialogOpening` events and waits up to the
//! global timeout for a dialog to appear.
mod common;
use common::{Response, execute, skip_if_no_chrome, spawn};
use earl_protocol_browser::PreparedBrowserCommand;
use earl_protocol_browser::schema::BrowserStep;
use std::collections::HashMap;

/// Test 7.1 — handle_dialog accepts an alert.
///
/// The page fires `alert('Hello')` 100 ms after load.  HandleDialog accepts
/// the pending dialog and the step succeeds with `{"ok": true}`.
#[tokio::test]
async fn handle_dialog_accepts_alert() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            r#"<html><body><p>loaded</p><script>window.onload = function() { setTimeout(function() { alert('Hello'); }, 100); };</script></body></html>"#,
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
            BrowserStep::HandleDialog {
                accept: true,
                prompt_text: None,
                optional: false,
            },
        ],
    };

    let result = execute(data)
        .await
        .expect("execute should succeed — dialog should be accepted");
    assert_eq!(
        result["ok"].as_bool(),
        Some(true),
        "HandleDialog result should be {{\"ok\": true}}; got: {result}"
    );
}

/// Test 7.2 — handle_dialog with prompt_text fills the prompt.
///
/// The page opens `window.prompt()` 100 ms after load and writes the
/// returned value to `document.title`.  HandleDialog accepts with `"my-answer"`
/// and the subsequent Evaluate step reads back the title.
#[tokio::test]
async fn handle_dialog_fills_prompt_text() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            r#"<html><body><script>window.onload = function() { setTimeout(function() { document.title = window.prompt('Enter value') || ''; }, 100); };</script></body></html>"#,
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
            BrowserStep::HandleDialog {
                accept: true,
                prompt_text: Some("my-answer".to_string()),
                optional: false,
            },
            // Short wait to let the JS callback finish setting document.title
            // after the dialog is dismissed.
            BrowserStep::WaitFor {
                time: Some(0.2),
                text: None,
                text_gone: None,
                timeout_ms: Some(2000),
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.title".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");
    assert_eq!(
        result["value"].as_str(),
        Some("my-answer"),
        "document.title should equal 'my-answer' after prompt is filled; got: {result}"
    );
}

/// Test 7.3 — handle_dialog rejects a confirm dialog.
///
/// The page opens `window.confirm()` 100 ms after load and sets
/// `document.body.textContent` to `"yes"` or `"no"`.  Dismissing with
/// `accept: false` must produce `"no"`.
#[tokio::test]
async fn handle_dialog_rejects_confirm() {
    if skip_if_no_chrome() {
        return;
    }

    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html(
            r#"<html><body><script>window.onload = function() { setTimeout(function() { document.body.textContent = window.confirm('Continue?') ? 'yes' : 'no'; }, 100); };</script></body></html>"#,
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
            BrowserStep::HandleDialog {
                accept: false,
                prompt_text: None,
                optional: false,
            },
            // Wait for the JS callback to update body.textContent after the
            // dialog is dismissed.
            BrowserStep::WaitFor {
                time: None,
                text: Some("no".to_string()),
                text_gone: None,
                timeout_ms: Some(2000),
                optional: false,
            },
            BrowserStep::Evaluate {
                function: "() => document.body.textContent.trim()".to_string(),
                r#ref: None,
                timeout_ms: None,
                optional: false,
            },
        ],
    };

    let result = execute(data).await.expect("execute should succeed");
    assert_eq!(
        result["value"].as_str(),
        Some("no"),
        "body text should be 'no' after confirm is rejected; got: {result}"
    );
}

/// Test 7.4 — HandleDialog times out when no dialog fires (Issue I4).
///
/// Navigate to a plain HTML page with no dialog, then call HandleDialog
/// immediately.  Because no dialog ever fires, the command should time out
/// and return an error mentioning "timeout".
#[tokio::test]
async fn handle_dialog_times_out_when_no_dialog_fires() {
    if skip_if_no_chrome() {
        return;
    }
    let _guard = common::chrome_lock().await;

    let mut routes = HashMap::new();
    routes.insert(
        "GET /".to_string(),
        Response::html("<html><body>no dialog here</body></html>"),
    );
    let server = spawn(routes).await;

    let data = PreparedBrowserCommand {
        session_id: None,
        headless: true,
        timeout_ms: 2_000, // short timeout so the test completes quickly
        on_failure_screenshot: false,
        steps: vec![
            BrowserStep::Navigate {
                url: server.url("/"),
                expected_status: None,
                timeout_ms: None,
                optional: false,
            },
            BrowserStep::HandleDialog {
                accept: true,
                prompt_text: None,
                optional: false,
            },
        ],
    };

    let err = execute(data)
        .await
        .expect_err("HandleDialog should time out when no dialog fires");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("timeout") || msg.contains("timed out") || msg.contains("time"),
        "error should mention timeout; got: {msg}"
    );
}
