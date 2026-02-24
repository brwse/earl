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
        let timeout_duration =
            std::time::Duration::from_millis(step.timeout_ms(global_timeout_ms));

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
    let path = std::env::temp_dir().join(format!(
        "earl-browser-failure-{}.png",
        chrono::Utc::now().timestamp_millis()
    ));
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        page.save_screenshot(
            chromiumoxide::page::ScreenshotParams::builder().build(),
            &path,
        ),
    )
    .await
    {
        Ok(Ok(_)) => eprintln!("diagnostic screenshot saved: {}", path.display()),
        _ => {} // Don't mask the original error.
    }
}

// ── Step dispatcher ────────────────────────────────────────────────────────────

pub async fn execute_step(ctx: &StepContext<'_>, step: &BrowserStep) -> Result<Value> {
    match step {
        BrowserStep::Navigate { url, expected_status, .. } => {
            step_navigate(ctx, url, *expected_status).await
        }
        BrowserStep::NavigateBack { .. } => step_navigate_back(ctx).await,
        BrowserStep::NavigateForward { .. } => step_navigate_forward(ctx).await,
        BrowserStep::Reload { .. } => step_reload(ctx).await,
        BrowserStep::Snapshot { .. } => step_snapshot(ctx).await,
        BrowserStep::Screenshot { path, full_page, .. } => {
            step_screenshot(ctx, path.as_deref(), Some(*full_page)).await
        }
        BrowserStep::Click { r#ref, selector, button: _, double_click, modifiers: _, .. } => {
            step_click(ctx, r#ref.as_deref(), selector.as_deref(), *double_click).await
        }
        BrowserStep::Hover { r#ref, selector, .. } => {
            step_hover(ctx, r#ref.as_deref(), selector.as_deref()).await
        }
        BrowserStep::Fill { r#ref, selector, text, submit, slowly: _, .. } => {
            step_fill(ctx, r#ref.as_deref(), selector.as_deref(), text, *submit).await
        }
        BrowserStep::SelectOption { r#ref, selector, values, .. } => {
            step_select_option(ctx, r#ref.as_deref(), selector.as_deref(), values).await
        }
        BrowserStep::PressKey { key, .. } => step_press_key(ctx, key).await,
        BrowserStep::Check { r#ref, selector, .. } => {
            step_set_checked(ctx, r#ref.as_deref(), selector.as_deref(), true).await
        }
        BrowserStep::Uncheck { r#ref, selector, .. } => {
            step_set_checked(ctx, r#ref.as_deref(), selector.as_deref(), false).await
        }
        BrowserStep::Drag { start_ref, start_selector, end_ref, end_selector, .. } => {
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
        BrowserStep::MouseDrag { start_x, start_y, end_x, end_y, .. } => {
            step_mouse_drag(ctx, *start_x, *start_y, *end_x, *end_y).await
        }
        BrowserStep::MouseDown { button, .. } => {
            step_mouse_button(ctx, button.as_deref(), true).await
        }
        BrowserStep::MouseUp { button, .. } => {
            step_mouse_button(ctx, button.as_deref(), false).await
        }
        BrowserStep::MouseWheel { delta_x, delta_y, .. } => {
            step_mouse_wheel(ctx, *delta_x, *delta_y).await
        }
        // All other steps: stub for now, implemented in Task 9.
        other => {
            tracing::warn!(
                "browser step '{}' is not yet implemented in this version",
                other.action_name()
            );
            Ok(json!({"ok": true}))
        }
    }
}

// ── Navigation ─────────────────────────────────────────────────────────────────

async fn step_navigate(
    ctx: &StepContext<'_>,
    url: &str,
    _expected_status: Option<u16>,
) -> Result<Value> {
    validate_url_scheme(url)?;

    ctx.page
        .goto(url)
        .await
        .map_err(|e| anyhow::anyhow!("navigate to {url} failed: {e}"))?;

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
    let mut node_map: HashMap<String, &chromiumoxide::cdp::browser_protocol::accessibility::AxNode> =
        HashMap::new();
    for n in &cdp_nodes {
        node_map.insert(n.node_id.inner().to_string(), n);
    }

    // Convert a CDP AxNode into our simplified AXNode (recursively).
    // The full tree can be large; we call the flat list version.
    // CDP `GetFullAXTree` returns all nodes flat with parent_id references.
    // Build the tree by finding root nodes (no parent_id) and recursing.
    fn build_tree(
        node_id_str: &str,
        node_map: &HashMap<String, &chromiumoxide::cdp::browser_protocol::accessibility::AxNode>,
    ) -> Option<AXNode> {
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
            .filter_map(|child_id| build_tree(child_id.inner(), node_map))
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
        .filter_map(|n| build_tree(n.node_id.inner(), &node_map))
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
    let out_path = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!(
                "earl-screenshot-{}.png",
                chrono::Utc::now().timestamp_millis()
            ))
        });

    let params = chromiumoxide::page::ScreenshotParams::builder()
        .full_page(full_page.unwrap_or(false))
        .build();

    ctx.page
        .save_screenshot(params, &out_path)
        .await
        .map_err(|e| anyhow::anyhow!("screenshot failed: {e}"))?;

    let bytes = tokio::fs::read(&out_path).await?;
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

    Ok(json!({
        "path": out_path.to_string_lossy(),
        "data": data,
    }))
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
    el.click().await.map_err(|e| anyhow::anyhow!("click failed: {e}"))?;
    if double_click {
        el.click()
            .await
            .map_err(|e| anyhow::anyhow!("double-click second click failed: {e}"))?;
    }
    Ok(json!({"ok": true}))
}

async fn step_hover(
    ctx: &StepContext<'_>,
    ref_: Option<&str>,
    selector: Option<&str>,
) -> Result<Value> {
    let el = find_element_by_selector(ctx, selector, ref_, "hover").await?;
    el.hover().await.map_err(|e| anyhow::anyhow!("hover failed: {e}"))?;
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
    el.click().await.map_err(|e| anyhow::anyhow!("fill click: {e}"))?;
    // Clear the existing value before typing.
    el.call_js_fn(
        "function() { this.value = ''; this.dispatchEvent(new Event('input', {bubbles: true})); }",
        false,
    )
    .await
    .map_err(|e| anyhow::anyhow!("fill clear value: {e}"))?;
    el.type_str(text).await.map_err(|e| anyhow::anyhow!("fill type_str: {e}"))?;
    if submit.unwrap_or(false) {
        el.press_key("Return").await.map_err(|e| anyhow::anyhow!("fill submit: {e}"))?;
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
    let current: Value = result
        .result
        .value
        .unwrap_or(Value::Bool(false));
    if current.as_bool() != Some(checked) {
        el.click().await.map_err(|e| anyhow::anyhow!("set_checked click: {e}"))?;
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
        let type_ = field.get("type").and_then(|v| v.as_str()).unwrap_or("textbox");
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

async fn step_mouse_wheel(
    ctx: &StepContext<'_>,
    delta_x: f64,
    delta_y: f64,
) -> Result<Value> {
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
    fn http_scheme_allowed() {
        assert!(validate_url_scheme("https://example.com").is_ok());
        assert!(validate_url_scheme("http://example.com/path?q=1").is_ok());
    }

    #[test]
    fn blob_uri_rejected() {
        assert!(validate_url_scheme("blob:https://example.com/abc").is_err());
    }
}
