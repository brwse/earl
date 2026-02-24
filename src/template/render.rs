use std::sync::LazyLock;

use anyhow::{Context, Result};
use minijinja::{Environment, UndefinedBehavior, Value as JinjaValue};
use serde_json::{Map, Value};

static JINJA_ENV: LazyLock<Environment<'static>> = LazyLock::new(|| {
    let mut env = Environment::new();
    // Chainable: accessing an absent optional param returns Undefined rather
    // than erroring. Template-level typos are caught at load time by
    // validate_template_args, so Chainable is safe at runtime.
    env.set_undefined_behavior(UndefinedBehavior::Chainable);
    env
});

pub fn render_json_value(value: &Value, context: &Value) -> Result<Value> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Bool(v) => Ok(Value::Bool(*v)),
        Value::Number(v) => Ok(Value::Number(v.clone())),
        Value::String(v) => render_string_value(v, context),
        Value::Array(values) => {
            let mut out = Vec::with_capacity(values.len());
            for item in values {
                out.push(render_json_value(item, context)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(obj) => {
            let mut out = Map::new();
            for (k, v) in obj {
                let rendered_key = render_string_raw(k, context)?;
                out.insert(rendered_key, render_json_value(v, context)?);
            }
            Ok(Value::Object(out))
        }
    }
}

pub fn render_string_raw(input: &str, context: &Value) -> Result<String> {
    let ctx = JinjaValue::from_serialize(context);
    JINJA_ENV
        .render_str(input, ctx)
        .with_context(|| format!("template render failed for string `{input}`"))
}

fn render_string_value(input: &str, context: &Value) -> Result<Value> {
    if let Some(expr_str) = pure_expression(input) {
        // Evaluate the expression directly to get a typed minijinja Value.
        // This cleanly handles:
        //   - Undefined (absent optional param) → Value::Null
        //   - None/null                          → Value::Null
        //   - Integer, float, bool, array, object → correct JSON type
        //   - Explicit empty string ""            → Value::String("") (preserved)
        let expr = JINJA_ENV
            .compile_expression(expr_str)
            .with_context(|| format!("template render failed for string `{input}`"))?;
        let result = expr
            .eval(context)
            .with_context(|| format!("template render failed for string `{input}`"))?;
        if result.is_undefined() {
            return Ok(Value::Null);
        }
        return serde_json::to_value(&result)
            .with_context(|| format!("template render failed for string `{input}`"));
    }
    let rendered = render_string_raw(input, context)?;
    Ok(Value::String(rendered))
}

/// If `input` is a single `{{ expr }}` with no nested braces, return the inner
/// expression string. Returns `None` for multi-expression strings or plain text.
fn pure_expression(input: &str) -> Option<&str> {
    let trimmed = input.trim();
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        let inner = &trimmed[2..trimmed.len() - 2];
        if !inner.contains("{{") {
            return Some(inner.trim());
        }
    }
    None
}
