use earl::output::human::render_human_output;
use earl::output::json::render_json_output;
use earl::protocol::executor::ExecutionResult;
use earl::template::schema::{ResultDecode, ResultTemplate};
use serde_json::{Map, json};

#[test]
fn result_alias_exposes_result_to_template_context() {
    let template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "id={{ issue.id }} q={{ args.query }}".to_string(),
        result_alias: Some("issue".to_string()),
    };

    let mut args = Map::new();
    args.insert("query".to_string(), json!("hello"));
    let result = json!({"id": 123});

    let out = render_human_output(&template, &args, &result).unwrap();
    assert_eq!(out, "id=123 q=hello");
}

fn default_execution() -> ExecutionResult {
    ExecutionResult {
        status: 200,
        url: "https://api.example.com".to_string(),
        result: json!({}),
        decoded: json!({}),
    }
}

#[test]
fn json_output_includes_status_code() {
    let out = render_json_output(&default_execution());
    assert_eq!(out["status"], json!(200));
}

#[test]
fn json_output_includes_url() {
    let out = render_json_output(&default_execution());
    assert_eq!(out["url"], json!("https://api.example.com"));
}

#[test]
fn json_output_includes_result() {
    let execution = ExecutionResult {
        result: json!({"ok": true}),
        ..default_execution()
    };
    let out = render_json_output(&execution);
    assert_eq!(out["result"]["ok"], json!(true));
}

#[test]
fn json_output_includes_decoded() {
    let execution = ExecutionResult {
        decoded: json!({"raw": "value"}),
        ..default_execution()
    };
    let out = render_json_output(&execution);
    assert_eq!(out["decoded"]["raw"], json!("value"));
}
