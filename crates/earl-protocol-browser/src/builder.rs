use anyhow::{Result, bail};
use earl_core::render::TemplateRenderer;
use serde_json::Value;

use crate::PreparedBrowserCommand;
use crate::schema::{BrowserOperationTemplate, BrowserStep};

const DEFAULT_SESSION_ID: &str = "default";

/// Build a `PreparedBrowserCommand` from a `BrowserOperationTemplate` by
/// rendering all Jinja template strings with the given context.
pub fn build_browser_request(
    op: &BrowserOperationTemplate,
    context: &Value,
    renderer: &dyn TemplateRenderer,
) -> Result<PreparedBrowserCommand> {
    let tmpl = &op.browser;

    if tmpl.steps.is_empty() {
        bail!("operation.browser.steps must not be empty");
    }

    let session_id = match tmpl.session_id.as_deref() {
        None => Some(DEFAULT_SESSION_ID.to_string()),
        Some(s) => {
            let rendered = renderer.render_str(s, context)?;
            if rendered.is_empty() { None } else { Some(rendered) }
        }
    };

    // Render all string fields in each step via the renderer.
    let steps: Vec<BrowserStep> = tmpl
        .steps
        .iter()
        .map(|step| render_step(step, context, renderer))
        .collect::<Result<_>>()?;

    Ok(PreparedBrowserCommand {
        session_id,
        headless: tmpl.headless,
        timeout_ms: tmpl.timeout_ms,
        on_failure_screenshot: tmpl.on_failure_screenshot,
        steps,
    })
}

/// Render all Jinja template strings within a step by round-tripping through
/// `serde_json::Value` and walking every string node through the renderer.
fn render_step(step: &BrowserStep, ctx: &Value, r: &dyn TemplateRenderer) -> Result<BrowserStep> {
    let mut val = serde_json::to_value(step)?;
    render_strings_in_value(&mut val, ctx, r)?;
    Ok(serde_json::from_value(val)?)
}

fn render_strings_in_value(v: &mut Value, ctx: &Value, r: &dyn TemplateRenderer) -> Result<()> {
    match v {
        Value::String(s) => *s = r.render_str(s, ctx)?,
        Value::Object(map) => {
            for (k, val) in map.iter_mut() {
                if k == "action" {
                    // Never render the serde tag discriminant - it must stay unchanged
                    continue;
                }
                render_strings_in_value(val, ctx, r)?;
            }
        }
        Value::Array(arr) => {
            for val in arr.iter_mut() {
                render_strings_in_value(val, ctx, r)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PassthroughRenderer;

    impl TemplateRenderer for PassthroughRenderer {
        fn render_str(&self, t: &str, _ctx: &Value) -> anyhow::Result<String> {
            Ok(t.to_string())
        }

        fn render_value(&self, v: &Value, _ctx: &Value) -> anyhow::Result<Value> {
            Ok(v.clone())
        }
    }

    #[test]
    fn build_renders_session_id_and_url() {
        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
                "browser": {
                    "session_id": "my-session",
                    "steps": [{"action":"navigate","url":"https://example.com"}]
                }
            }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({"args": {}, "secrets": {}});
        let cmd = build_browser_request(&op, &ctx, &PassthroughRenderer).unwrap();
        assert_eq!(cmd.session_id.as_deref(), Some("my-session"));
        assert_eq!(cmd.steps.len(), 1);
    }

    #[test]
    fn empty_steps_returns_error() {
        let op: crate::schema::BrowserOperationTemplate =
            serde_json::from_str(r#"{"browser": {"steps": []}}"#).unwrap();
        let ctx = serde_json::json!({});
        assert!(build_browser_request(&op, &ctx, &PassthroughRenderer).is_err());
    }

    #[test]
    fn empty_session_id_becomes_none() {
        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
                "browser": {
                    "session_id": "",
                    "steps": [{"action":"snapshot"}]
                }
            }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({});
        let cmd = build_browser_request(&op, &ctx, &PassthroughRenderer).unwrap();
        assert!(cmd.session_id.is_none());
    }

    #[test]
    fn absent_session_id_defaults_to_default() {
        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
                "browser": {
                    "steps": [{"action":"snapshot"}]
                }
            }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({});
        let cmd = build_browser_request(&op, &ctx, &PassthroughRenderer).unwrap();
        assert_eq!(cmd.session_id.as_deref(), Some(super::DEFAULT_SESSION_ID));
    }

    // Setting session_id = "" is the explicit opt-out from the default session.
    #[test]
    fn explicit_empty_session_id_is_ephemeral() {
        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
                "browser": {
                    "session_id": "",
                    "steps": [{"action":"snapshot"}]
                }
            }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({});
        let cmd = build_browser_request(&op, &ctx, &PassthroughRenderer).unwrap();
        assert!(cmd.session_id.is_none());
    }

    #[test]
    fn build_preserves_headless_and_timeout() {
        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
                "browser": {
                    "headless": false,
                    "timeout_ms": 60000,
                    "on_failure_screenshot": false,
                    "steps": [{"action":"snapshot"}]
                }
            }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({});
        let cmd = build_browser_request(&op, &ctx, &PassthroughRenderer).unwrap();
        assert!(!cmd.headless);
        assert_eq!(cmd.timeout_ms, 60000);
        assert!(!cmd.on_failure_screenshot);
    }

    #[test]
    fn render_step_strings_are_passed_through_renderer() {
        /// Renderer that only transforms non-action strings by uppercasing them.
        /// We use a targeted substitution so we can distinguish rendered from
        /// original values without corrupting the serde `action` discriminant.
        struct UppercaseUrlRenderer;
        impl TemplateRenderer for UppercaseUrlRenderer {
            fn render_str(&self, t: &str, _ctx: &Value) -> anyhow::Result<String> {
                // Only transform strings that look like URLs (contain "://").
                if t.contains("://") {
                    Ok(t.to_uppercase())
                } else {
                    Ok(t.to_string())
                }
            }
            fn render_value(&self, v: &Value, _ctx: &Value) -> anyhow::Result<Value> {
                Ok(v.clone())
            }
        }

        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
                "browser": {
                    "steps": [{"action":"navigate","url":"https://example.com"}]
                }
            }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({});
        let cmd = build_browser_request(&op, &ctx, &UppercaseUrlRenderer).unwrap();
        // The url field is a string containing "://" so it gets uppercased.
        if let crate::schema::BrowserStep::Navigate { url, .. } = &cmd.steps[0] {
            assert_eq!(url, "HTTPS://EXAMPLE.COM");
        } else {
            panic!("expected Navigate step");
        }
    }

    #[test]
    fn render_does_not_corrupt_action_discriminant() {
        struct UppercaseRenderer;
        impl earl_core::TemplateRenderer for UppercaseRenderer {
            fn render_str(&self, t: &str, _ctx: &Value) -> anyhow::Result<String> {
                Ok(t.to_uppercase())
            }
            fn render_value(&self, v: &Value, _ctx: &Value) -> anyhow::Result<Value> {
                Ok(v.clone())
            }
        }
        let op: crate::schema::BrowserOperationTemplate = serde_json::from_str(
            r#"{
            "browser": {
                "steps": [{"action":"navigate","url":"https://example.com"}]
            }
        }"#,
        )
        .unwrap();
        let ctx = serde_json::json!({});
        // This should NOT fail — action discriminant must be preserved
        let cmd = build_browser_request(&op, &ctx, &UppercaseRenderer).unwrap();
        assert_eq!(cmd.steps.len(), 1);
        // URL should be uppercased
        if let crate::schema::BrowserStep::Navigate { url, .. } = &cmd.steps[0] {
            assert_eq!(url, "HTTPS://EXAMPLE.COM");
        } else {
            panic!("expected Navigate step");
        }
    }
}
