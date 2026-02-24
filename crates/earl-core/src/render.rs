use std::collections::BTreeMap;

use anyhow::Result;
use serde_json::Value;

/// Trait abstracting template rendering so protocol crates can render
/// strings and JSON values without depending on a specific engine.
pub trait TemplateRenderer {
    /// Render a template string with the given context, returning the raw string.
    fn render_str(&self, template: &str, context: &Value) -> Result<String>;

    /// Render a JSON value, recursively expanding any template strings.
    fn render_value(&self, value: &Value, context: &Value) -> Result<Value>;
}

/// Render a `BTreeMap<String, Value>` of template key-value pairs into a
/// flat list of `(String, String)` pairs, expanding arrays into multiple
/// entries with the same key.
pub fn render_key_value_map(
    input: Option<&BTreeMap<String, Value>>,
    context: &Value,
    renderer: &dyn TemplateRenderer,
) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    let Some(input) = input else {
        return Ok(out);
    };

    for (key, value) in input {
        let rendered_key = renderer.render_str(key, context)?;
        let rendered_value = renderer.render_value(value, context)?;

        match rendered_value {
            Value::Null => {} // Absent optional params render to null; skip them so they are omitted from the request.
            Value::Array(values) => {
                for value in values {
                    let s = value_to_string(value)?;
                    if !s.is_empty() {
                        out.push((rendered_key.clone(), s));
                    }
                }
            }
            other => {
                let s = value_to_string(other)?;
                // Empty strings are treated as absent in query/header maps — same
                // policy as null, since `default = ""` was the old workaround for
                // optional params that are now handled via null-skipping.
                if !s.is_empty() {
                    out.push((rendered_key, s));
                }
            }
        }
    }

    Ok(out)
}

/// Convert a `serde_json::Value` into its string representation.
pub fn value_to_string(value: Value) -> Result<String> {
    let out = match value {
        Value::Null => String::new(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::String(v) => v,
        Value::Array(_) | Value::Object(_) => serde_json::to_string(&value)?,
    };
    Ok(out)
}
