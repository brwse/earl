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
        // All other steps: stub for now, implemented in Tasks 8 and 9.
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
    use chromiumoxide::cdp::browser_protocol::accessibility::AxNodeId;
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
