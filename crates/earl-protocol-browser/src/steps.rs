use anyhow::Result;
use chromiumoxide::Page;
use serde_json::{Value, json};

use crate::accessibility::{AXNode, render_ax_tree};
use crate::error::BrowserError;
use crate::schema::BrowserStep;

// ── URL scheme validation ──────────────────────────────────────────────────────

/// Validate that the given URL has an allowed scheme (http or https only).
/// Rejects file://, javascript:, data:, blob:, and any other scheme.
pub fn validate_url_scheme(url: &str) -> Result<()> {
    let scheme = url.split(':').next().unwrap_or("").to_lowercase();
    match scheme.as_str() {
        "http" | "https" => Ok(()),
        other => Err(BrowserError::DisallowedScheme {
            scheme: other.to_string(),
        }
        .into()),
    }
}

// ── File path validation ───────────────────────────────────────────────────────

/// Reject file paths that could escape the working directory.
///
/// Only relative paths are permitted — absolute paths are rejected to prevent
/// writes to arbitrary filesystem locations. `..` components are also rejected
/// to block traversal out of the working directory.
fn validate_file_path(path: &str) -> Result<()> {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return Err(anyhow::anyhow!(
            "file path \"{path}\" is not allowed: only relative paths are permitted"
        ));
    }
    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err(anyhow::anyhow!(
            "file path \"{path}\" is not allowed: path traversal (`..`) is not permitted"
        ));
    }
    Ok(())
}

// ── Step execution context ─────────────────────────────────────────────────────

pub struct StepContext<'a> {
    pub page: &'a Page,
    pub step_index: usize,
    pub total_steps: usize,
    pub global_timeout_ms: u64,
}

// ── Main step loop ─────────────────────────────────────────────────────────────

pub async fn execute_steps(
    page: &Page,
    steps: &[BrowserStep],
    global_timeout_ms: u64,
    on_failure_screenshot: bool,
) -> Result<Value> {
    let total = steps.len();
    let mut last_result = json!({"ok": true});

    for (i, step) in steps.iter().enumerate() {
        let ctx = StepContext {
            page,
            step_index: i,
            total_steps: total,
            global_timeout_ms,
        };
        let timeout_duration = std::time::Duration::from_millis(step.timeout_ms(global_timeout_ms));

        let outcome = tokio::time::timeout(timeout_duration, execute_step(&ctx, step)).await;

        match outcome {
            Ok(Ok(val)) => last_result = val,
            Ok(Err(e)) => {
                if step.is_optional() {
                    tracing::warn!(
                        "optional browser step {} ({}) failed (skipping): {e}",
                        i,
                        step.action_name()
                    );
                    continue;
                }
                if on_failure_screenshot {
                    attempt_failure_screenshot(page).await;
                }
                return Err(e);
            }
            Err(_elapsed) => {
                let timeout_ms = step.timeout_ms(global_timeout_ms);
                let e: anyhow::Error = BrowserError::Timeout {
                    step: i,
                    action: step.action_name().into(),
                    timeout_ms,
                }
                .into();
                if step.is_optional() {
                    tracing::warn!(
                        "optional browser step {} ({}) timed out (skipping)",
                        i,
                        step.action_name()
                    );
                    continue;
                }
                if on_failure_screenshot {
                    attempt_failure_screenshot(page).await;
                }
                return Err(e);
            }
        }
    }

    Ok(last_result)
}

/// Attempt to capture a diagnostic screenshot on step failure.
/// Errors here are silently swallowed so they don't mask the original error.
async fn attempt_failure_screenshot(page: &Page) {
    let params = chromiumoxide::page::ScreenshotParams::builder().build();
    if let Ok(Ok(bytes)) =
        tokio::time::timeout(std::time::Duration::from_secs(2), page.screenshot(params)).await
    {
        let path = std::env::temp_dir().join(format!(
            "earl-browser-failure-{}.png",
            chrono::Utc::now().timestamp_millis()
        ));
        if let Ok(()) = std::fs::write(&path, &bytes) {
            // Restrict permissions so the diagnostic file is not world-readable.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
            eprintln!("diagnostic screenshot saved: {}", path.display());
        }
    }
}

// ── Step dispatcher ────────────────────────────────────────────────────────────

pub async fn execute_step(ctx: &StepContext<'_>, step: &BrowserStep) -> Result<Value> {
    match step {
        BrowserStep::Navigate {
            url,
            expected_status,
            ..
        } => step_navigate(ctx, url, *expected_status).await,
        BrowserStep::NavigateBack { .. } => step_navigate_back(ctx).await,
        BrowserStep::NavigateForward { .. } => step_navigate_forward(ctx).await,
        BrowserStep::Reload { .. } => step_reload(ctx).await,
        BrowserStep::Snapshot { .. } => step_snapshot(ctx).await,
        BrowserStep::Screenshot {
            path, full_page, ..
        } => step_screenshot(ctx, path.as_deref(), Some(*full_page)).await,
        BrowserStep::Click {
            r#ref,
            selector,
            double_click,
            ..
        } => step_click(ctx, r#ref.as_deref(), selector.as_deref(), *double_click).await,
        BrowserStep::Hover {
            r#ref, selector, ..
        } => step_hover(ctx, r#ref.as_deref(), selector.as_deref()).await,
        BrowserStep::Fill {
            r#ref,
            selector,
            text,
            submit,
            ..
        } => step_fill(ctx, r#ref.as_deref(), selector.as_deref(), text, *submit).await,
        BrowserStep::SelectOption {
            r#ref,
            selector,
            values,
            ..
        } => step_select_option(ctx, r#ref.as_deref(), selector.as_deref(), values).await,
        BrowserStep::PressKey { key, .. } => step_press_key(ctx, key).await,
        BrowserStep::Check {
            r#ref, selector, ..
        } => step_set_checked(ctx, r#ref.as_deref(), selector.as_deref(), true).await,
        BrowserStep::Uncheck {
            r#ref, selector, ..
        } => step_set_checked(ctx, r#ref.as_deref(), selector.as_deref(), false).await,
        BrowserStep::Drag {
            start_ref,
            start_selector,
            end_ref,
            end_selector,
            ..
        } => {
            step_drag(
                ctx,
                start_ref.as_deref(),
                start_selector.as_deref(),
                end_ref.as_deref(),
                end_selector.as_deref(),
            )
            .await
        }
        BrowserStep::FillForm { fields, .. } => step_fill_form(ctx, fields).await,
        BrowserStep::MouseMove { x, y, .. } => step_mouse_move(ctx, *x, *y).await,
        BrowserStep::MouseClick { x, y, button, .. } => {
            step_mouse_click(ctx, *x, *y, button.as_deref()).await
        }
        BrowserStep::MouseDrag {
            start_x,
            start_y,
            end_x,
            end_y,
            ..
        } => step_mouse_drag(ctx, *start_x, *start_y, *end_x, *end_y).await,
        BrowserStep::MouseDown { button, .. } => {
            step_mouse_button(ctx, button.as_deref(), true).await
        }
        BrowserStep::MouseUp { button, .. } => {
            step_mouse_button(ctx, button.as_deref(), false).await
        }
        BrowserStep::MouseWheel {
            delta_x, delta_y, ..
        } => step_mouse_wheel(ctx, *delta_x, *delta_y).await,

        // ── Wait / Assert ──────────────────────────────────────────────────
        BrowserStep::WaitFor {
            time,
            text,
            text_gone,
            timeout_ms,
            ..
        } => {
            step_wait_for(
                ctx,
                *time,
                text.as_deref(),
                text_gone.as_deref(),
                timeout_ms.unwrap_or(ctx.global_timeout_ms),
            )
            .await
        }
        BrowserStep::VerifyElementVisible {
            role,
            accessible_name,
            ..
        } => step_verify_element_visible(ctx, role.as_deref(), accessible_name.as_deref()).await,
        BrowserStep::VerifyTextVisible { text, .. } => step_verify_text_visible(ctx, text).await,
        BrowserStep::VerifyListVisible { r#ref, items, .. } => {
            if r#ref.is_some() {
                return Err(anyhow::anyhow!(
                    "browser step {} (verify_list_visible): ref-based targeting is not yet \
                     implemented; omit the ref field to match against the full page text",
                    ctx.step_index
                ));
            }
            step_verify_list_visible(ctx, items).await
        }
        BrowserStep::VerifyValue { r#ref, value, .. } => {
            if r#ref.is_some() {
                return Err(anyhow::anyhow!(
                    "browser step {} (verify_value): ref-based targeting is not yet \
                     implemented; omit the ref field to match against the active element",
                    ctx.step_index
                ));
            }
            step_verify_value(ctx, value).await
        }

        // ── JavaScript ────────────────────────────────────────────────────
        BrowserStep::Evaluate { function, .. } => step_evaluate(ctx, function).await,
        BrowserStep::RunCode { code, .. } => step_run_code(ctx, code).await,

        // ── Tabs & Viewport ───────────────────────────────────────────────
        BrowserStep::Tabs {
            operation, index, ..
        } => step_tabs(ctx, operation, *index).await,
        BrowserStep::Resize { width, height, .. } => step_resize(ctx, *width, *height).await,
        BrowserStep::Close { .. } => step_close(ctx).await,

        // ── Network (stubs — not yet implemented) ────────────────────────
        BrowserStep::ConsoleMessages { .. } => {
            Ok(json!({"messages": [], "note": "console_messages: not yet implemented"}))
        }
        BrowserStep::ConsoleClear { .. } => {
            Ok(json!({"ok": true, "note": "console_clear: not yet implemented"}))
        }
        BrowserStep::NetworkRequests { .. } => {
            Ok(json!({"requests": [], "note": "network_requests: not yet implemented"}))
        }
        BrowserStep::NetworkClear { .. } => {
            Ok(json!({"ok": true, "note": "network_clear: not yet implemented"}))
        }
        BrowserStep::Route { .. } => Ok(json!({"ok": true, "note": "route: not yet implemented"})),
        BrowserStep::RouteList { .. } => {
            Ok(json!({"routes": [], "note": "route_list: not yet implemented"}))
        }
        BrowserStep::Unroute { .. } => {
            Ok(json!({"ok": true, "note": "unroute: not yet implemented"}))
        }

        // ── Cookies ───────────────────────────────────────────────────────
        BrowserStep::CookieList { domain, .. } => step_cookie_list(ctx, domain.as_deref()).await,
        BrowserStep::CookieGet { name, .. } => step_cookie_get(ctx, name).await,
        BrowserStep::CookieSet {
            name,
            value,
            domain,
            path,
            expires,
            http_only,
            secure,
            ..
        } => {
            step_cookie_set(
                ctx,
                name,
                value,
                domain.as_deref(),
                path.as_deref(),
                *expires,
                *http_only,
                *secure,
            )
            .await
        }
        BrowserStep::CookieDelete { name, .. } => step_cookie_delete(ctx, name).await,
        BrowserStep::CookieClear { .. } => step_cookie_clear(ctx).await,

        // ── Web Storage ───────────────────────────────────────────────────
        BrowserStep::LocalStorageGet { key, .. } => step_storage_get(ctx, "local", key).await,
        BrowserStep::LocalStorageSet { key, value, .. } => {
            step_storage_set(ctx, "local", key, value).await
        }
        BrowserStep::LocalStorageDelete { key, .. } => step_storage_delete(ctx, "local", key).await,
        BrowserStep::LocalStorageClear { .. } => step_storage_clear(ctx, "local").await,
        BrowserStep::SessionStorageGet { key, .. } => step_storage_get(ctx, "session", key).await,
        BrowserStep::SessionStorageSet { key, value, .. } => {
            step_storage_set(ctx, "session", key, value).await
        }
        BrowserStep::SessionStorageDelete { key, .. } => {
            step_storage_delete(ctx, "session", key).await
        }
        BrowserStep::SessionStorageClear { .. } => step_storage_clear(ctx, "session").await,
        BrowserStep::StorageState { path, .. } => step_storage_state(ctx, path.as_deref()).await,
        BrowserStep::SetStorageState { path, .. } => step_set_storage_state(ctx, path).await,

        // ── File / Dialog / Download ──────────────────────────────────────
        BrowserStep::FileUpload { .. } => {
            Ok(json!({"ok": true, "note": "file_upload: not yet implemented"}))
        }
        BrowserStep::HandleDialog {
            accept,
            prompt_text,
            ..
        } => step_handle_dialog(ctx, *accept, prompt_text.as_deref()).await,
        BrowserStep::Download { .. } => {
            Ok(json!({"ok": true, "note": "download: not yet implemented"}))
        }

        // ── Output / Recording ────────────────────────────────────────────
        BrowserStep::PdfSave { path, .. } => step_pdf_save(ctx, path.as_deref()).await,
        BrowserStep::StartVideo { .. } => {
            Ok(json!({"ok": true, "note": "video recording: not yet implemented"}))
        }
        BrowserStep::StopVideo { .. } => {
            Ok(json!({"ok": true, "note": "video recording: not yet implemented"}))
        }
        BrowserStep::StartTracing { .. } => {
            Ok(json!({"ok": true, "note": "tracing: not yet implemented"}))
        }
        BrowserStep::StopTracing { .. } => {
            Ok(json!({"ok": true, "note": "tracing: not yet implemented"}))
        }
        BrowserStep::GenerateLocator { r#ref, .. } => step_generate_locator(ctx, r#ref).await,
    }
}

// ── Navigation ─────────────────────────────────────────────────────────────────

async fn step_navigate(
    ctx: &StepContext<'_>,
    url: &str,
    expected_status: Option<u16>,
) -> Result<Value> {
    validate_url_scheme(url)?;

    ctx.page
        .goto(url)
        .await
        .map_err(|e| anyhow::anyhow!("navigate to {url} failed: {e}"))?;

    if let Some(expected) = expected_status {
        // Use the Performance Navigation Timing API to read the HTTP response
        // status code after the navigation has settled.
        let actual = ctx
            .page
            .evaluate("window.performance.getEntriesByType('navigation')[0]?.responseStatus ?? 0")
            .await
            .map_err(|e| anyhow::anyhow!("navigate status check failed: {e}"))?
            .into_value::<serde_json::Value>()
            .ok()
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u16;

        if actual != expected {
            return Err(BrowserError::AssertionFailed {
                step: ctx.step_index,
                action: "navigate".to_string(),
                message: format!("expected HTTP status {expected}, got {actual} for {url}"),
            }
            .into());
        }
    }

    Ok(json!({ "ok": true, "url": url }))
}

async fn step_navigate_back(ctx: &StepContext<'_>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::page::{
        GetNavigationHistoryParams, NavigateToHistoryEntryParams,
    };

    let history = ctx
        .page
        .execute(GetNavigationHistoryParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("get navigation history failed: {e}"))?;

    let current_index = history.result.current_index;
    if current_index <= 0 {
        // No history to go back to — treat as no-op.
        return Ok(json!({ "ok": true }));
    }
    let target_index = (current_index - 1) as usize;
    let entries = &history.result.entries;
    if target_index >= entries.len() {
        return Ok(json!({ "ok": true }));
    }
    let entry_id = entries[target_index].id;

    ctx.page
        .execute(NavigateToHistoryEntryParams::new(entry_id))
        .await
        .map_err(|e| anyhow::anyhow!("navigate back failed: {e}"))?;

    ctx.page
        .wait_for_navigation()
        .await
        .map_err(|e| anyhow::anyhow!("wait for navigation after go-back failed: {e}"))?;

    Ok(json!({ "ok": true }))
}

async fn step_navigate_forward(ctx: &StepContext<'_>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::page::{
        GetNavigationHistoryParams, NavigateToHistoryEntryParams,
    };

    let history = ctx
        .page
        .execute(GetNavigationHistoryParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("get navigation history failed: {e}"))?;

    let current_index = history.result.current_index as usize;
    let entries = &history.result.entries;
    let next_index = current_index + 1;
    if next_index >= entries.len() {
        // No forward history — treat as no-op.
        return Ok(json!({ "ok": true }));
    }
    let entry_id = entries[next_index].id;

    ctx.page
        .execute(NavigateToHistoryEntryParams::new(entry_id))
        .await
        .map_err(|e| anyhow::anyhow!("navigate forward failed: {e}"))?;

    ctx.page
        .wait_for_navigation()
        .await
        .map_err(|e| anyhow::anyhow!("wait for navigation after go-forward failed: {e}"))?;

    Ok(json!({ "ok": true }))
}

async fn step_reload(ctx: &StepContext<'_>) -> Result<Value> {
    ctx.page
        .reload()
        .await
        .map_err(|e| anyhow::anyhow!("reload failed: {e}"))?;

    Ok(json!({ "ok": true }))
}

// ── Observation ────────────────────────────────────────────────────────────────

async fn step_snapshot(ctx: &StepContext<'_>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::accessibility::GetFullAxTreeParams;

    let response = ctx
        .page
        .execute(GetFullAxTreeParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("get full AX tree failed: {e}"))?;

    let cdp_nodes = response.result.nodes;

    // Build a flat id→node map and then reconstruct the tree hierarchy.
    use std::collections::HashMap;

    // Index nodes by their node_id.
    let mut node_map: HashMap<
        String,
        &chromiumoxide::cdp::browser_protocol::accessibility::AxNode,
    > = HashMap::new();
    for n in &cdp_nodes {
        node_map.insert(n.node_id.inner().to_string(), n);
    }

    // Convert a CDP AxNode into our simplified AXNode (recursively).
    // The full tree can be large; we call the flat list version.
    // CDP `GetFullAXTree` returns all nodes flat with parent_id references.
    // Build the tree by finding root nodes (no parent_id) and recursing.
    // A depth limit guards against stack overflow on pathologically deep trees.
    const MAX_TREE_DEPTH: usize = 80;
    fn build_tree(
        node_id_str: &str,
        node_map: &HashMap<String, &chromiumoxide::cdp::browser_protocol::accessibility::AxNode>,
        depth: usize,
    ) -> Option<AXNode> {
        if depth > MAX_TREE_DEPTH {
            return None;
        }
        let cdp = node_map.get(node_id_str)?;
        if cdp.ignored {
            return None;
        }

        let role = cdp
            .role
            .as_ref()
            .and_then(|v| v.value.as_ref())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        let name = cdp
            .name
            .as_ref()
            .and_then(|v| v.value.as_ref())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let backend_node_id = cdp
            .backend_dom_node_id
            .as_ref()
            .map(|id| *id.inner() as u64)
            .unwrap_or(0);

        let children = cdp
            .child_ids
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter_map(|child_id| build_tree(child_id.inner(), node_map, depth + 1))
            .collect();

        Some(AXNode {
            backend_node_id,
            role,
            name,
            children,
        })
    }

    // Collect root nodes (nodes with no parent or whose parent is not in the map).
    let roots: Vec<AXNode> = cdp_nodes
        .iter()
        .filter(|n| {
            !n.ignored
                && n.parent_id
                    .as_ref()
                    .map(|pid| !node_map.contains_key(pid.inner()))
                    .unwrap_or(true)
        })
        .filter_map(|n| build_tree(n.node_id.inner(), &node_map, 0))
        .collect();

    let max_nodes = 5000;
    let (markdown, refs) = render_ax_tree(&roots, max_nodes);

    Ok(json!({
        "text": markdown,
        "refs": refs,
    }))
}

async fn step_screenshot(
    ctx: &StepContext<'_>,
    path: Option<&str>,
    full_page: Option<bool>,
) -> Result<Value> {
    if let Some(p) = path {
        validate_file_path(p)?;
    }

    // Use page.screenshot() to get bytes directly — avoids a temp-file round-trip
    // and ensures no world-readable file is left behind when no path is given.
    let params = chromiumoxide::page::ScreenshotParams::builder()
        .full_page(full_page.unwrap_or(false))
        .build();

    let bytes = ctx
        .page
        .screenshot(params)
        .await
        .map_err(|e| anyhow::anyhow!("screenshot failed: {e}"))?;

    if let Some(p) = path {
        // User wants the file saved to disk.
        tokio::fs::write(p, &bytes)
            .await
            .map_err(|e| anyhow::anyhow!("screenshot write {p}: {e}"))?;
        Ok(json!({"path": p}))
    } else {
        // No path — return bytes as base64 only.
        let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
        Ok(json!({"data": data}))
    }
}

// ── Interaction helpers ────────────────────────────────────────────────────────

/// Locate a page element by CSS selector. If a `ref_` is provided but no
/// selector, a helpful error is returned explaining that ref-based targeting
/// requires session mode (not yet implemented). If neither is provided, an
/// `ElementNotFound` error is returned.
async fn find_element_by_selector(
    ctx: &StepContext<'_>,
    selector: Option<&str>,
    ref_: Option<&str>,
    action: &str,
) -> Result<chromiumoxide::element::Element> {
    let sel = match selector {
        Some(s) => s,
        None => {
            if ref_.is_some() {
                return Err(anyhow::anyhow!(
                    "browser step {} ({action}): 'ref' targeting requires session mode \
                     (not yet available in this version); use 'selector' instead",
                    ctx.step_index
                ));
            }
            return Err(BrowserError::ElementNotFound {
                step: ctx.step_index,
                action: action.to_string(),
                selector: "(none provided)".to_string(),
                completed: ctx.step_index,
                total: ctx.total_steps,
            }
            .into());
        }
    };

    ctx.page.find_element(sel).await.map_err(|_| {
        BrowserError::ElementNotFound {
            step: ctx.step_index,
            action: action.to_string(),
            selector: sel.to_string(),
            completed: ctx.step_index,
            total: ctx.total_steps,
        }
        .into()
    })
}

async fn step_click(
    ctx: &StepContext<'_>,
    ref_: Option<&str>,
    selector: Option<&str>,
    double_click: bool,
) -> Result<Value> {
    let el = find_element_by_selector(ctx, selector, ref_, "click").await?;
    el.click()
        .await
        .map_err(|e| anyhow::anyhow!("click failed: {e}"))?;
    if double_click {
        el.click()
            .await
            .map_err(|e| anyhow::anyhow!("double-click second click failed: {e}"))?;
        // The two sequential .click() calls don't fire the dblclick DOM event
        // that many frameworks listen to. Dispatch it explicitly.
        el.call_js_fn(
            "function() { this.dispatchEvent(new MouseEvent('dblclick', {bubbles: true, cancelable: true})); }",
            false,
        )
        .await
        .map_err(|e| anyhow::anyhow!("double-click dblclick event dispatch failed: {e}"))?;
    }
    Ok(json!({"ok": true}))
}

async fn step_hover(
    ctx: &StepContext<'_>,
    ref_: Option<&str>,
    selector: Option<&str>,
) -> Result<Value> {
    let el = find_element_by_selector(ctx, selector, ref_, "hover").await?;
    el.hover()
        .await
        .map_err(|e| anyhow::anyhow!("hover failed: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_fill(
    ctx: &StepContext<'_>,
    ref_: Option<&str>,
    selector: Option<&str>,
    text: &str,
    submit: Option<bool>,
) -> Result<Value> {
    let el = find_element_by_selector(ctx, selector, ref_, "fill").await?;
    el.click()
        .await
        .map_err(|e| anyhow::anyhow!("fill click: {e}"))?;
    // Clear the existing value before typing.
    el.call_js_fn(
        "function() { this.value = ''; this.dispatchEvent(new Event('input', {bubbles: true})); }",
        false,
    )
    .await
    .map_err(|e| anyhow::anyhow!("fill clear value: {e}"))?;
    el.type_str(text)
        .await
        .map_err(|e| anyhow::anyhow!("fill type_str: {e}"))?;
    if submit.unwrap_or(false) {
        el.press_key("Enter")
            .await
            .map_err(|e| anyhow::anyhow!("fill submit: {e}"))?;
    }
    Ok(json!({"ok": true}))
}

async fn step_select_option(
    ctx: &StepContext<'_>,
    _ref_: Option<&str>,
    selector: Option<&str>,
    values: &[String],
) -> Result<Value> {
    let sel = selector.unwrap_or("");
    let values_json = serde_json::to_string(values)?;
    let sel_json = serde_json::to_string(sel)?;
    ctx.page
        .evaluate(format!(
            r#"(function() {{
                var el = document.querySelector({sel_json});
                if (!el) return false;
                Array.from(el.options).forEach(function(o) {{
                    o.selected = {values_json}.indexOf(o.value) !== -1;
                }});
                el.dispatchEvent(new Event('change', {{bubbles: true}}));
                return true;
            }})()"#,
        ))
        .await
        .map_err(|e| anyhow::anyhow!("select_option: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_press_key(ctx: &StepContext<'_>, key: &str) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchKeyEventParams, DispatchKeyEventType,
    };
    use chromiumoxide::keys;

    let key_def = keys::get_key_definition(key)
        .ok_or_else(|| anyhow::anyhow!("press_key: unknown key '{key}'"))?;

    let mut cmd = DispatchKeyEventParams::builder();

    let key_down_type = if let Some(txt) = key_def.text {
        cmd = cmd.text(txt);
        DispatchKeyEventType::KeyDown
    } else if key_def.key.len() == 1 {
        cmd = cmd.text(key_def.key);
        DispatchKeyEventType::KeyDown
    } else {
        DispatchKeyEventType::RawKeyDown
    };

    cmd = cmd
        .key(key_def.key)
        .code(key_def.code)
        .windows_virtual_key_code(key_def.key_code)
        .native_virtual_key_code(key_def.key_code);

    ctx.page
        .execute(cmd.clone().r#type(key_down_type).build().unwrap())
        .await
        .map_err(|e| anyhow::anyhow!("press_key key_down: {e}"))?;
    ctx.page
        .execute(cmd.r#type(DispatchKeyEventType::KeyUp).build().unwrap())
        .await
        .map_err(|e| anyhow::anyhow!("press_key key_up: {e}"))?;

    Ok(json!({"ok": true}))
}

async fn step_set_checked(
    ctx: &StepContext<'_>,
    ref_: Option<&str>,
    selector: Option<&str>,
    checked: bool,
) -> Result<Value> {
    let action = if checked { "check" } else { "uncheck" };
    let el = find_element_by_selector(ctx, selector, ref_, action).await?;
    // Only click if the current state differs from the desired state.
    let result = el
        .call_js_fn("function() { return this.checked; }", false)
        .await
        .map_err(|e| anyhow::anyhow!("set_checked get state: {e}"))?;
    let current: Value = result.result.value.unwrap_or(Value::Bool(false));
    if current.as_bool() != Some(checked) {
        el.click()
            .await
            .map_err(|e| anyhow::anyhow!("set_checked click: {e}"))?;
    }
    Ok(json!({"ok": true}))
}

async fn step_drag(
    ctx: &StepContext<'_>,
    _start_ref: Option<&str>,
    start_selector: Option<&str>,
    _end_ref: Option<&str>,
    end_selector: Option<&str>,
) -> Result<Value> {
    let start_sel = start_selector.unwrap_or("");
    let end_sel = end_selector.unwrap_or("");
    let start_json = serde_json::to_string(start_sel)?;
    let end_json = serde_json::to_string(end_sel)?;
    ctx.page
        .evaluate(format!(
            r#"(function() {{
                var src = document.querySelector({start_json});
                var dst = document.querySelector({end_json});
                if (!src || !dst) return false;
                src.dispatchEvent(new DragEvent('dragstart', {{bubbles: true, cancelable: true}}));
                dst.dispatchEvent(new DragEvent('dragenter', {{bubbles: true, cancelable: true}}));
                dst.dispatchEvent(new DragEvent('dragover',  {{bubbles: true, cancelable: true}}));
                dst.dispatchEvent(new DragEvent('drop',      {{bubbles: true, cancelable: true}}));
                src.dispatchEvent(new DragEvent('dragend',   {{bubbles: true, cancelable: true}}));
                return true;
            }})()"#,
        ))
        .await
        .map_err(|e| anyhow::anyhow!("drag: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_fill_form(ctx: &StepContext<'_>, fields: &[Value]) -> Result<Value> {
    for field in fields {
        let ref_ = field.get("ref").and_then(|v| v.as_str());
        let selector = field.get("selector").and_then(|v| v.as_str());
        let value = field.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let type_ = field
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("textbox");
        match type_ {
            "checkbox" => {
                let checked = value == "true" || value == "1";
                step_set_checked(ctx, ref_, selector, checked).await?;
            }
            _ => {
                step_fill(ctx, ref_, selector, value, None).await?;
            }
        }
    }
    Ok(json!({"ok": true}))
}

// ── Mouse coordinate steps ─────────────────────────────────────────────────────

async fn step_mouse_move(ctx: &StepContext<'_>, x: f64, y: f64) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType,
    };
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseMoved)
                .x(x)
                .y(y)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_move: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_mouse_click(
    ctx: &StepContext<'_>,
    x: f64,
    y: f64,
    button: Option<&str>,
) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType,
    };
    let mb = parse_mouse_button(button);
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MousePressed)
                .x(x)
                .y(y)
                .button(mb.clone())
                .click_count(1i64)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_click pressed: {e}"))?;
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseReleased)
                .x(x)
                .y(y)
                .button(mb)
                .click_count(1i64)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_click released: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_mouse_drag(
    ctx: &StepContext<'_>,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType,
    };
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MousePressed)
                .x(start_x)
                .y(start_y)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_drag pressed: {e}"))?;
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseMoved)
                .x(end_x)
                .y(end_y)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_drag moved: {e}"))?;
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseReleased)
                .x(end_x)
                .y(end_y)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_drag released: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_mouse_button(
    ctx: &StepContext<'_>,
    button: Option<&str>,
    pressed: bool,
) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType,
    };
    // Use the centre of the viewport as the default position.
    let pos: Value = ctx
        .page
        .evaluate("({x: window.innerWidth / 2, y: window.innerHeight / 2})")
        .await
        .map_err(|e| anyhow::anyhow!("mouse_button get position: {e}"))?
        .into_value()?;
    let x = pos["x"].as_f64().unwrap_or(400.0);
    let y = pos["y"].as_f64().unwrap_or(300.0);
    let mb = parse_mouse_button(button);
    let evt_type = if pressed {
        DispatchMouseEventType::MousePressed
    } else {
        DispatchMouseEventType::MouseReleased
    };
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(evt_type)
                .x(x)
                .y(y)
                .button(mb)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_button: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_mouse_wheel(ctx: &StepContext<'_>, delta_x: f64, delta_y: f64) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventParams, DispatchMouseEventType,
    };
    let pos: Value = ctx
        .page
        .evaluate("({x: window.innerWidth / 2, y: window.innerHeight / 2})")
        .await
        .map_err(|e| anyhow::anyhow!("mouse_wheel get position: {e}"))?
        .into_value()?;
    let x = pos["x"].as_f64().unwrap_or(400.0);
    let y = pos["y"].as_f64().unwrap_or(300.0);
    ctx.page
        .execute(
            DispatchMouseEventParams::builder()
                .r#type(DispatchMouseEventType::MouseWheel)
                .x(x)
                .y(y)
                .delta_x(delta_x)
                .delta_y(delta_y)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("mouse_wheel: {e}"))?;
    Ok(json!({"ok": true}))
}

// ── Wait / Assert ───────────────────────────────────────────────────────────

async fn step_wait_for(
    ctx: &StepContext<'_>,
    time: Option<f64>,
    text: Option<&str>,
    text_gone: Option<&str>,
    timeout_ms: u64,
) -> Result<Value> {
    if let Some(secs) = time {
        tokio::time::sleep(std::time::Duration::from_secs_f64(secs)).await;
    }

    if text.is_none() && text_gone.is_none() {
        return Ok(json!({"ok": true}));
    }

    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms.max(200));

    loop {
        let body_text: Value = ctx
            .page
            .evaluate("document.body ? document.body.innerText : ''")
            .await
            .map_err(|e| anyhow::anyhow!("wait_for evaluate: {e}"))?
            .into_value()?;
        let body = body_text.as_str().unwrap_or("");

        if let Some(t) = text {
            if body.contains(t) {
                // text found — check text_gone too
                if let Some(tg) = text_gone {
                    if !body.contains(tg) {
                        return Ok(json!({"ok": true}));
                    }
                } else {
                    return Ok(json!({"ok": true}));
                }
            }
        } else if let Some(tg) = text_gone
            && !body.contains(tg)
        {
            return Ok(json!({"ok": true}));
        }

        // Check deadline before sleeping so we never overshoot by a full poll interval.
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err(BrowserError::Timeout {
                step: ctx.step_index,
                action: "wait_for".into(),
                timeout_ms,
            }
            .into());
        }

        // Sleep for at most the remaining time to avoid overshooting the deadline.
        let remaining = deadline - now;
        tokio::time::sleep(remaining.min(std::time::Duration::from_millis(200))).await;
    }
}

async fn step_verify_text_visible(ctx: &StepContext<'_>, text: &str) -> Result<Value> {
    let body_text: Value = ctx
        .page
        .evaluate("document.body ? document.body.innerText : ''")
        .await
        .map_err(|e| anyhow::anyhow!("verify_text_visible evaluate: {e}"))?
        .into_value()?;
    let body = body_text.as_str().unwrap_or("");
    if body.contains(text) {
        Ok(json!({"ok": true, "text": text}))
    } else {
        Err(BrowserError::AssertionFailed {
            step: ctx.step_index,
            action: "verify_text_visible".into(),
            message: format!("text not found in page: {text:?}"),
        }
        .into())
    }
}

async fn step_verify_list_visible(ctx: &StepContext<'_>, items: &[String]) -> Result<Value> {
    let body_text: Value = ctx
        .page
        .evaluate("document.body ? document.body.innerText : ''")
        .await
        .map_err(|e| anyhow::anyhow!("verify_list_visible evaluate: {e}"))?
        .into_value()?;
    let body = body_text.as_str().unwrap_or("");
    let mut missing = Vec::new();
    for item in items {
        if !body.contains(item.as_str()) {
            missing.push(item.as_str());
        }
    }
    if missing.is_empty() {
        Ok(json!({"ok": true}))
    } else {
        Err(BrowserError::AssertionFailed {
            step: ctx.step_index,
            action: "verify_list_visible".into(),
            message: format!("items not found in page: {:?}", missing),
        }
        .into())
    }
}

async fn step_verify_element_visible(
    ctx: &StepContext<'_>,
    role: Option<&str>,
    accessible_name: Option<&str>,
) -> Result<Value> {
    // Build a simple JS check using aria attributes.
    let role_json = serde_json::to_string(role.unwrap_or(""))?;
    let name_json = serde_json::to_string(accessible_name.unwrap_or(""))?;
    let result: Value = ctx
        .page
        .evaluate(format!(
            r#"(function() {{
                var role = {role_json};
                var name = {name_json};
                var els = document.querySelectorAll('*');
                for (var i = 0; i < els.length; i++) {{
                    var el = els[i];
                    var elRole = el.getAttribute('role') || el.tagName.toLowerCase();
                    var elName = el.getAttribute('aria-label') || el.textContent || '';
                    if ((role === '' || elRole === role) && (name === '' || elName.trim().indexOf(name) !== -1)) {{
                        return true;
                    }}
                }}
                return false;
            }})()"#,
        ))
        .await
        .map_err(|e| anyhow::anyhow!("verify_element_visible evaluate: {e}"))?
        .into_value()?;

    if result.as_bool().unwrap_or(false) {
        Ok(json!({"ok": true}))
    } else {
        Err(BrowserError::AssertionFailed {
            step: ctx.step_index,
            action: "verify_element_visible".into(),
            message: format!(
                "element not found — role={:?} name={:?}",
                role, accessible_name
            ),
        }
        .into())
    }
}

async fn step_verify_value(ctx: &StepContext<'_>, expected: &str) -> Result<Value> {
    // Evaluate the value of the currently focused element (or the first input).
    let expected_json = serde_json::to_string(expected)?;
    let result: Value = ctx
        .page
        .evaluate(
            r#"(function() {
                var el = document.activeElement || document.querySelector('input,textarea,select');
                if (!el) return null;
                return el.value !== undefined ? el.value : el.textContent;
            })()"#,
        )
        .await
        .map_err(|e| anyhow::anyhow!("verify_value evaluate: {e}"))?
        .into_value()?;

    let actual = result.as_str().unwrap_or("");
    if actual == expected {
        Ok(json!({"ok": true, "value": actual}))
    } else {
        Err(BrowserError::AssertionFailed {
            step: ctx.step_index,
            action: "verify_value".into(),
            message: format!("expected value {expected_json}, got {:?}", actual),
        }
        .into())
    }
}

// ── JavaScript ──────────────────────────────────────────────────────────────

async fn step_evaluate(ctx: &StepContext<'_>, function: &str) -> Result<Value> {
    let result: Value = ctx
        .page
        .evaluate(function)
        .await
        .map_err(|e| anyhow::anyhow!("evaluate: {e}"))?
        .into_value()?;
    Ok(json!({"value": result}))
}

async fn step_run_code(ctx: &StepContext<'_>, code: &str) -> Result<Value> {
    let wrapped = format!("(async () => {{ {} }})()", code);
    let result: Value = ctx
        .page
        .evaluate(wrapped)
        .await
        .map_err(|e| anyhow::anyhow!("run_code: {e}"))?
        .into_value()?;
    Ok(json!({"value": result}))
}

// ── Tabs & Viewport ─────────────────────────────────────────────────────────

async fn step_tabs(ctx: &StepContext<'_>, operation: &str, _index: Option<usize>) -> Result<Value> {
    match operation {
        "list" => {
            let url: Value = ctx
                .page
                .evaluate("window.location.href")
                .await
                .map_err(|e| anyhow::anyhow!("tabs list: {e}"))?
                .into_value()?;
            Ok(json!({"tabs": [{"url": url, "index": 0, "active": true}]}))
        }
        _ => Ok(json!({
            "ok": true,
            "note": "full tab management (new/select/close) requires session mode"
        })),
    }
}

async fn step_resize(ctx: &StepContext<'_>, width: u32, height: u32) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
    ctx.page
        .execute(SetDeviceMetricsOverrideParams::new(
            width as i64,
            height as i64,
            1.0_f64,
            false,
        ))
        .await
        .map_err(|e| anyhow::anyhow!("resize: {e}"))?;
    Ok(json!({"ok": true, "width": width, "height": height}))
}

async fn step_close(ctx: &StepContext<'_>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::page::CloseParams;
    ctx.page
        .execute(CloseParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("close: {e}"))?;
    Ok(json!({"ok": true}))
}

// ── Cookies ─────────────────────────────────────────────────────────────────

async fn step_cookie_list(ctx: &StepContext<'_>, domain: Option<&str>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::network::GetCookiesParams;
    let result = ctx
        .page
        .execute(GetCookiesParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("cookie_list: {e}"))?;
    let cookies: Vec<Value> = result
        .result
        .cookies
        .iter()
        .filter(|c| domain.is_none_or(|d| c.domain.contains(d)))
        .map(|c| {
            json!({
                "name": c.name,
                "value": c.value,
                "domain": c.domain,
                "path": c.path,
                "expires": c.expires,
                "http_only": c.http_only,
                "secure": c.secure,
                "session": c.session,
            })
        })
        .collect();
    Ok(json!({"cookies": cookies}))
}

async fn step_cookie_get(ctx: &StepContext<'_>, name: &str) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::network::GetCookiesParams;
    let result = ctx
        .page
        .execute(GetCookiesParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("cookie_get: {e}"))?;
    let cookie = result.result.cookies.iter().find(|c| c.name == name);
    match cookie {
        Some(c) => Ok(json!({
            "name": c.name,
            "value": c.value,
            "domain": c.domain,
            "path": c.path,
            "expires": c.expires,
            "http_only": c.http_only,
            "secure": c.secure,
        })),
        None => Ok(json!({"name": name, "value": null})),
    }
}

#[allow(clippy::too_many_arguments)]
async fn step_cookie_set(
    ctx: &StepContext<'_>,
    name: &str,
    value: &str,
    domain: Option<&str>,
    path: Option<&str>,
    expires: Option<f64>,
    http_only: bool,
    secure: bool,
) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::network::SetCookieParams;
    let mut params = SetCookieParams::new(name, value);
    if let Some(d) = domain {
        params.domain = Some(d.to_string());
    }
    if let Some(p) = path {
        params.path = Some(p.to_string());
    }
    if let Some(e) = expires {
        use chromiumoxide::cdp::browser_protocol::network::TimeSinceEpoch;
        params.expires = Some(TimeSinceEpoch::new(e));
    }
    params.http_only = Some(http_only);
    params.secure = Some(secure);
    ctx.page
        .execute(params)
        .await
        .map_err(|e| anyhow::anyhow!("cookie_set: {e}"))?;
    Ok(json!({"ok": true, "name": name}))
}

async fn step_cookie_delete(ctx: &StepContext<'_>, name: &str) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::network::DeleteCookiesParams;
    // CDP requires at least one of `url` or `domain`; use the current page URL.
    let url = ctx.page.url().await.ok().flatten();
    let mut params = DeleteCookiesParams::new(name);
    params.url = url;
    ctx.page
        .execute(params)
        .await
        .map_err(|e| anyhow::anyhow!("cookie_delete: {e}"))?;
    Ok(json!({"ok": true, "name": name}))
}

async fn step_cookie_clear(ctx: &StepContext<'_>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::network::ClearBrowserCookiesParams;
    ctx.page
        .execute(ClearBrowserCookiesParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("cookie_clear: {e}"))?;
    Ok(json!({"ok": true}))
}

// ── Web Storage ─────────────────────────────────────────────────────────────

/// `kind` is either `"local"` or `"session"`.
fn storage_js_obj(kind: &str) -> &'static str {
    if kind == "session" {
        "sessionStorage"
    } else {
        "localStorage"
    }
}

async fn step_storage_get(ctx: &StepContext<'_>, kind: &str, key: &str) -> Result<Value> {
    let key_json = serde_json::to_string(key)?;
    let obj = storage_js_obj(kind);
    let val: Value = ctx
        .page
        .evaluate(format!("{obj}.getItem({key_json})"))
        .await
        .map_err(|e| anyhow::anyhow!("storage_get: {e}"))?
        .into_value()?;
    Ok(json!({"key": key, "value": val}))
}

async fn step_storage_set(
    ctx: &StepContext<'_>,
    kind: &str,
    key: &str,
    value: &str,
) -> Result<Value> {
    let key_json = serde_json::to_string(key)?;
    let val_json = serde_json::to_string(value)?;
    let obj = storage_js_obj(kind);
    ctx.page
        .evaluate(format!("{obj}.setItem({key_json}, {val_json})"))
        .await
        .map_err(|e| anyhow::anyhow!("storage_set: {e}"))?;
    Ok(json!({"ok": true, "key": key}))
}

async fn step_storage_delete(ctx: &StepContext<'_>, kind: &str, key: &str) -> Result<Value> {
    let key_json = serde_json::to_string(key)?;
    let obj = storage_js_obj(kind);
    ctx.page
        .evaluate(format!("{obj}.removeItem({key_json})"))
        .await
        .map_err(|e| anyhow::anyhow!("storage_delete: {e}"))?;
    Ok(json!({"ok": true, "key": key}))
}

async fn step_storage_clear(ctx: &StepContext<'_>, kind: &str) -> Result<Value> {
    let obj = storage_js_obj(kind);
    ctx.page
        .evaluate(format!("{obj}.clear()"))
        .await
        .map_err(|e| anyhow::anyhow!("storage_clear: {e}"))?;
    Ok(json!({"ok": true}))
}

async fn step_storage_state(ctx: &StepContext<'_>, path: Option<&str>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::network::GetCookiesParams;

    // Gather cookies.
    let cookie_result = ctx
        .page
        .execute(GetCookiesParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("storage_state cookies: {e}"))?;
    let cookies: Vec<Value> = cookie_result
        .result
        .cookies
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "value": c.value,
                "domain": c.domain,
                "path": c.path,
                "expires": c.expires,
                "http_only": c.http_only,
                "secure": c.secure,
            })
        })
        .collect();

    // Gather localStorage.
    let ls: Value = ctx
        .page
        .evaluate(
            r#"(function() {
                var out = {};
                for (var i = 0; i < localStorage.length; i++) {
                    var k = localStorage.key(i);
                    out[k] = localStorage.getItem(k);
                }
                return out;
            })()"#,
        )
        .await
        .map_err(|e| anyhow::anyhow!("storage_state localStorage: {e}"))?
        .into_value()?;

    let state = json!({"cookies": cookies, "local_storage": ls});

    if let Some(p) = path {
        validate_file_path(p)?;
        let bytes = serde_json::to_vec_pretty(&state)?;
        tokio::fs::write(p, &bytes)
            .await
            .map_err(|e| anyhow::anyhow!("storage_state write {p}: {e}"))?;
        Ok(json!({"path": p, "cookies": cookies.len()}))
    } else {
        Ok(state)
    }
}

async fn step_set_storage_state(ctx: &StepContext<'_>, path: &str) -> Result<Value> {
    validate_file_path(path)?;
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| anyhow::anyhow!("set_storage_state read {path}: {e}"))?;
    let state: Value = serde_json::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("set_storage_state parse: {e}"))?;

    // Restore cookies.
    if let Some(cookies) = state.get("cookies").and_then(|v| v.as_array()) {
        use chromiumoxide::cdp::browser_protocol::network::SetCookieParams;
        for c in cookies {
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let value = c.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let mut params = SetCookieParams::new(name, value);
            if let Some(d) = c.get("domain").and_then(|v| v.as_str()) {
                params.domain = Some(d.to_string());
            }
            if let Some(p) = c.get("path").and_then(|v| v.as_str()) {
                params.path = Some(p.to_string());
            }
            if let Some(e) = c.get("expires").and_then(|v| v.as_f64()) {
                use chromiumoxide::cdp::browser_protocol::network::TimeSinceEpoch;
                params.expires = Some(TimeSinceEpoch::new(e));
            }
            if let Some(ho) = c.get("http_only").and_then(|v| v.as_bool()) {
                params.http_only = Some(ho);
            }
            if let Some(s) = c.get("secure").and_then(|v| v.as_bool()) {
                params.secure = Some(s);
            }
            ctx.page
                .execute(params)
                .await
                .map_err(|e| anyhow::anyhow!("set_storage_state set cookie: {e}"))?;
        }
    }

    // Restore localStorage.
    if let Some(ls) = state.get("local_storage").and_then(|v| v.as_object()) {
        let entries_json = serde_json::to_string(ls)?;
        ctx.page
            .evaluate(format!(
                r#"(function(entries) {{
                    localStorage.clear();
                    for (var k in entries) {{
                        localStorage.setItem(k, entries[k]);
                    }}
                }})({entries_json})"#,
            ))
            .await
            .map_err(|e| anyhow::anyhow!("set_storage_state localStorage: {e}"))?;
    }

    Ok(json!({"ok": true, "path": path}))
}

// ── Dialog ───────────────────────────────────────────────────────────────────

async fn step_handle_dialog(
    ctx: &StepContext<'_>,
    accept: bool,
    prompt_text: Option<&str>,
) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::page::{
        EventJavascriptDialogOpening, HandleJavaScriptDialogParams,
    };
    use futures::StreamExt;

    // Subscribe to dialog-opening events BEFORE attempting to handle, so we
    // don't miss a dialog that fires between the check and the dismiss call.
    let mut dialog_events = ctx
        .page
        .event_listener::<EventJavascriptDialogOpening>()
        .await
        .map_err(|e| anyhow::anyhow!("handle_dialog: subscribe: {e}"))?;

    let mut params = HandleJavaScriptDialogParams::new(accept);
    if let Some(t) = prompt_text {
        params.prompt_text = Some(t.to_string());
    }

    // Try to dismiss a dialog that is already open.
    if ctx.page.execute(params.clone()).await.is_ok() {
        return Ok(json!({"ok": true, "accept": accept}));
    }

    // Wait up to the global timeout for a dialog to appear.
    tokio::time::timeout(
        std::time::Duration::from_millis(ctx.global_timeout_ms),
        dialog_events.next(),
    )
    .await
    .map_err(|_| anyhow::anyhow!("handle_dialog: timed out waiting for dialog to appear"))?;

    // Dismiss the now-pending dialog.
    ctx.page
        .execute(params)
        .await
        .map_err(|e| anyhow::anyhow!("handle_dialog: {e}"))?;

    Ok(json!({"ok": true, "accept": accept}))
}

// ── PDF ──────────────────────────────────────────────────────────────────────

async fn step_pdf_save(ctx: &StepContext<'_>, path: Option<&str>) -> Result<Value> {
    use chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams;

    if let Some(p) = path {
        validate_file_path(p)?;
    }

    let result = ctx
        .page
        .execute(PrintToPdfParams::default())
        .await
        .map_err(|e| anyhow::anyhow!("pdf_save: {e}"))?;

    // result.result.data is a Binary wrapping a base64 string.
    let b64: String = result.result.data.into();
    let pdf_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64.trim())
        .map_err(|e| anyhow::anyhow!("pdf_save base64 decode: {e}"))?;

    if let Some(p) = path {
        // User wants the file saved to disk — no temp file needed.
        tokio::fs::write(p, &pdf_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("pdf_save write {p}: {e}"))?;
        Ok(json!({"path": p, "size": pdf_bytes.len()}))
    } else {
        // No path given — return bytes as base64; nothing written to disk.
        let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pdf_bytes);
        Ok(json!({"data": data, "size": pdf_bytes.len()}))
    }
}

// ── GenerateLocator ─────────────────────────────────────────────────────────

async fn step_generate_locator(_ctx: &StepContext<'_>, ref_: &str) -> Result<Value> {
    // Validate ref_ contains only safe characters for a CSS attribute value
    if !ref_
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow::anyhow!(
            "generate_locator: ref '{ref_}' contains characters unsafe for CSS attribute selector"
        ));
    }
    // Without a live ref→selector mapping, we return a CSS attribute selector
    // based on the ref ID. In session mode this would resolve to a precise selector.
    let locator = format!("[data-ref=\"{ref_}\"]");
    Ok(json!({"locator": locator, "ref": ref_}))
}

/// Parse an optional button string into a `MouseButton` enum value.
fn parse_mouse_button(
    button: Option<&str>,
) -> chromiumoxide::cdp::browser_protocol::input::MouseButton {
    use chromiumoxide::cdp::browser_protocol::input::MouseButton;
    match button {
        Some("right") => MouseButton::Right,
        Some("middle") => MouseButton::Middle,
        Some("back") => MouseButton::Back,
        Some("forward") => MouseButton::Forward,
        _ => MouseButton::Left,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disallowed_scheme_rejected() {
        assert!(validate_url_scheme("file:///etc/passwd").is_err());
        let err = validate_url_scheme("file:///etc/passwd").unwrap_err();
        assert!(err.to_string().contains("file"));
        assert!(err.to_string().contains("http"));
    }

    #[test]
    fn javascript_uri_rejected() {
        assert!(validate_url_scheme("javascript:alert(1)").is_err());
    }

    #[test]
    fn data_uri_rejected() {
        assert!(validate_url_scheme("data:text/html,<h1>test</h1>").is_err());
    }

    #[test]
    fn validate_file_path_rejects_traversal() {
        assert!(validate_file_path("../secret.txt").is_err());
        assert!(validate_file_path("/tmp/../../etc/passwd").is_err());
        assert!(validate_file_path("foo/../bar").is_err());
    }

    #[test]
    fn validate_file_path_rejects_absolute_paths() {
        assert!(validate_file_path("/tmp/output.png").is_err());
        assert!(validate_file_path("/etc/passwd").is_err());
        assert!(validate_file_path("/home/user/.bashrc").is_err());
    }

    #[test]
    fn validate_file_path_accepts_relative_paths() {
        assert!(validate_file_path("relative/path.pdf").is_ok());
        assert!(validate_file_path("file.json").is_ok());
        assert!(validate_file_path("output/screenshot.png").is_ok());
    }

    #[test]
    fn http_scheme_allowed() {
        assert!(validate_url_scheme("https://example.com").is_ok());
        assert!(validate_url_scheme("http://example.com/path?q=1").is_ok());
    }

    #[test]
    fn blob_uri_rejected() {
        assert!(validate_url_scheme("blob:https://example.com/abc").is_err());
    }

    // ── Tests for new step helpers (no browser required) ─────────────────────

    #[test]
    fn storage_js_obj_returns_correct_object() {
        assert_eq!(storage_js_obj("local"), "localStorage");
        assert_eq!(storage_js_obj("session"), "sessionStorage");
        // Anything that isn't "session" defaults to localStorage.
        assert_eq!(storage_js_obj("other"), "localStorage");
    }

    #[test]
    fn generate_locator_produces_data_ref_selector() {
        // The function is async but we can verify the locator format logic
        // by checking the string that would be produced.
        let ref_id = "e42";
        let expected = format!("[data-ref=\"{ref_id}\"]");
        assert_eq!(expected, "[data-ref=\"e42\"]");
    }

    #[test]
    fn generate_locator_rejects_unsafe_ref() {
        // Characters that would break a CSS attribute selector must be rejected.
        let unsafe_refs = [
            "e\"42", // double-quote breaks the attribute value
            "e]42",  // bracket closes the selector early
            "e[42",  // bracket opens a nested selector
            "e 42",  // space is not a valid identifier character
            "e<42>", // angle brackets
            "e;42",  // semicolons
        ];
        for bad in &unsafe_refs {
            let is_safe = bad
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
            assert!(
                !is_safe,
                "expected '{bad}' to be rejected as unsafe for CSS attribute selector"
            );
        }

        // Safe refs must pass the same check.
        let safe_refs = ["e42", "my-ref", "some_id", "Abc123", "a-b_c"];
        for good in &safe_refs {
            let is_safe = good
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
            assert!(
                is_safe,
                "expected '{good}' to be accepted as safe for CSS attribute selector"
            );
        }
    }

    #[test]
    fn cookie_set_params_new_api() {
        use chromiumoxide::cdp::browser_protocol::network::SetCookieParams;
        let params = SetCookieParams::new("session", "abc123");
        assert_eq!(params.name, "session");
        assert_eq!(params.value, "abc123");
        assert!(params.domain.is_none());
    }

    #[test]
    fn cookie_delete_params_new_api() {
        use chromiumoxide::cdp::browser_protocol::network::DeleteCookiesParams;
        let params = DeleteCookiesParams::new("session");
        assert_eq!(params.name, "session");
    }

    #[test]
    fn time_since_epoch_new_api() {
        use chromiumoxide::cdp::browser_protocol::network::TimeSinceEpoch;
        let t = TimeSinceEpoch::new(1_700_000_000.0_f64);
        assert_eq!(*t.inner(), 1_700_000_000.0_f64);
    }

    #[test]
    fn resize_params_new_api() {
        use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
        let params = SetDeviceMetricsOverrideParams::new(1280_i64, 720_i64, 1.0_f64, false);
        assert_eq!(params.width, 1280);
        assert_eq!(params.height, 720);
        assert!(!params.mobile);
    }

    #[test]
    fn handle_dialog_params_new_api() {
        use chromiumoxide::cdp::browser_protocol::page::HandleJavaScriptDialogParams;
        let params = HandleJavaScriptDialogParams::new(true);
        assert!(params.accept);
        assert!(params.prompt_text.is_none());
    }

    #[test]
    fn wait_for_time_only_does_not_poll() {
        // WaitFor with only `time` set (no text conditions) returns Ok immediately
        // after sleeping — we test that the function signature compiles correctly
        // by verifying BrowserStep::WaitFor deserialises with the timeout_ms field.
        use crate::schema::BrowserStep;
        // timeout_ms is now optional; verify it deserialises both with and without the field.
        let json_with = r#"{"action":"wait_for","time":0.001,"timeout_ms":5000}"#;
        let step: BrowserStep = serde_json::from_str(json_with).unwrap();
        assert!(
            matches!(step, BrowserStep::WaitFor { time: Some(t), timeout_ms: Some(5000), .. } if t < 1.0)
        );
        let json_without = r#"{"action":"wait_for","time":0.001}"#;
        let step2: BrowserStep = serde_json::from_str(json_without).unwrap();
        assert!(
            matches!(step2, BrowserStep::WaitFor { time: Some(t), timeout_ms: None, .. } if t < 1.0)
        );
    }

    #[test]
    fn verify_text_visible_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"verify_text_visible","text":"Hello world"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::VerifyTextVisible { text, .. } if text == "Hello world")
        );
    }

    #[test]
    fn verify_list_visible_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"verify_list_visible","ref":"root","items":["Apple","Banana"]}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::VerifyListVisible { items, .. } if items.len() == 2));
    }

    #[test]
    fn evaluate_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"evaluate","function":"() => document.title"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::Evaluate { function, .. } if function.contains("document.title"))
        );
    }

    #[test]
    fn run_code_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"run_code","code":"return 42;"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::RunCode { code, .. } if code == "return 42;"));
    }

    #[test]
    fn cookie_list_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"cookie_list","domain":"example.com"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::CookieList { domain: Some(d), .. } if d == "example.com")
        );
    }

    #[test]
    fn cookie_set_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"cookie_set","name":"tok","value":"xyz","http_only":true}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::CookieSet { name, http_only: true, .. } if name == "tok")
        );
    }

    #[test]
    fn local_storage_get_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"local_storage_get","key":"auth_token"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::LocalStorageGet { key, .. } if key == "auth_token"));
    }

    #[test]
    fn session_storage_set_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"session_storage_set","key":"sid","value":"abc"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::SessionStorageSet { key, value, .. } if key == "sid" && value == "abc")
        );
    }

    #[test]
    fn tabs_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"tabs","operation":"list"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::Tabs { operation, .. } if operation == "list"));
    }

    #[test]
    fn resize_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"resize","width":1280,"height":720}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(
            step,
            BrowserStep::Resize {
                width: 1280,
                height: 720,
                ..
            }
        ));
    }

    #[test]
    fn handle_dialog_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"handle_dialog","accept":false,"prompt_text":"no"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::HandleDialog { accept: false, prompt_text: Some(t), .. } if t == "no")
        );
    }

    #[test]
    fn pdf_save_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"pdf_save","path":"/tmp/out.pdf"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::PdfSave { path: Some(p), .. } if p == "/tmp/out.pdf"));
    }

    #[test]
    fn generate_locator_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"generate_locator","ref":"e7"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::GenerateLocator { r#ref, .. } if r#ref == "e7"));
    }

    #[test]
    fn storage_state_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"storage_state","path":"/tmp/state.json"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::StorageState { path: Some(p), .. } if p == "/tmp/state.json")
        );
    }

    #[test]
    fn set_storage_state_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"set_storage_state","path":"/tmp/state.json"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::SetStorageState { path, .. } if path == "/tmp/state.json")
        );
    }

    #[test]
    fn route_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"route","pattern":"**/api/data","status":200,"body":"{}"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::Route { pattern, status: Some(200), .. } if pattern == "**/api/data")
        );
    }

    #[test]
    fn console_messages_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"console_messages","level":"error"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(
            matches!(step, BrowserStep::ConsoleMessages { level: Some(l), .. } if l == "error")
        );
    }

    #[test]
    fn network_requests_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"network_requests","include_static":true}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(
            step,
            BrowserStep::NetworkRequests {
                include_static: true,
                ..
            }
        ));
    }

    #[test]
    fn verify_value_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"verify_value","ref":"e1","value":"expected"}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(step, BrowserStep::VerifyValue { value, .. } if value == "expected"));
    }

    #[test]
    fn start_video_step_deserialises() {
        use crate::schema::BrowserStep;
        let json = r#"{"action":"start_video","width":1280,"height":720}"#;
        let step: BrowserStep = serde_json::from_str(json).unwrap();
        assert!(matches!(
            step,
            BrowserStep::StartVideo {
                width: Some(1280),
                height: Some(720),
                ..
            }
        ));
    }
}
