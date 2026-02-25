use earl::template::render::{render_json_value, render_string_raw};
use serde_json::json;

#[test]
fn pure_expression_returns_typed_value() {
    let context = json!({"args": {"count": 42}});
    let rendered = render_json_value(&json!("{{ args.count }}"), &context).unwrap();
    assert_eq!(rendered, json!(42));
}

#[test]
fn renders_mixed_text_as_string() {
    let context = json!({"args": {"name": "world"}});
    let rendered = render_json_value(&json!("hello {{ args.name }}"), &context).unwrap();
    assert_eq!(rendered, json!("hello world"));
}

#[test]
fn undefined_variable_renders_as_empty_string() {
    // Chainable undefined behavior: absent args render as empty string in raw
    // string templates rather than erroring. Type-faithful rendering via
    // pure_expression (render_json_value) maps undefined to null instead.
    let context = json!({"args": {}});
    let result = render_string_raw("{{ args.missing }}", &context).unwrap();
    assert_eq!(result, "");
}

#[test]
fn expression_in_object_key_evaluates_to_context_value() {
    let context = json!({"args": {"key": "x-id"}});
    let value = json!({"{{ args.key }}": "static"});
    let rendered = render_json_value(&value, &context).unwrap();
    assert_eq!(rendered, json!({"x-id": "static"}));
}

#[test]
fn pure_expression_value_preserves_numeric_type() {
    // Pure expression rendering preserves context types: integer 123 stays integer.
    let context = json!({"args": {"value": 123}});
    let value = json!({"key": "{{ args.value }}"});
    let rendered = render_json_value(&value, &context).unwrap();
    assert_eq!(rendered, json!({"key": 123}));
}

#[test]
fn skips_null_values_in_rendered_objects() {
    // Absent optional params render to null and are omitted from the object,
    // preventing { "title": null } in PATCH bodies when only some fields are set.
    let context = json!({"args": {"state": "closed"}});
    let value = json!({"state": "{{ args.state }}", "title": "{{ args.title }}"});
    let rendered = render_json_value(&value, &context).unwrap();
    assert_eq!(rendered, json!({"state": "closed"}));
}
