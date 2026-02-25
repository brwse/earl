use earl::expression::cli_args::{parse_cli_args, CliArgsError};
use earl::template::schema::{ParamSpec, ParamType};

fn s(v: &str) -> String {
    v.to_string()
}

fn args(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|v| s(v)).collect()
}

fn search_params() -> Vec<ParamSpec> {
    vec![
        ParamSpec {
            name: "query".to_string(),
            r#type: ParamType::String,
            required: true,
            default: None,
            description: None,
        },
        ParamSpec {
            name: "per_page".to_string(),
            r#type: ParamType::Integer,
            required: false,
            default: Some(serde_json::json!(20)),
            description: None,
        },
        ParamSpec {
            name: "verbose".to_string(),
            r#type: ParamType::Boolean,
            required: false,
            default: Some(serde_json::json!(false)),
            description: None,
        },
    ]
}

#[test]
fn string_param_is_parsed_as_string_value() {
    let params = search_params();
    let expr = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "repo:rust-lang/rust", "--per_page", "5"]),
        &params,
    )
    .unwrap();

    assert_eq!(
        expr.named_args[0],
        (
            "query".to_string(),
            serde_json::json!("repo:rust-lang/rust")
        )
    );
}

#[test]
fn integer_param_is_coerced_from_string_input() {
    let params = search_params();
    let expr = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "repo:rust-lang/rust", "--per_page", "5"]),
        &params,
    )
    .unwrap();

    assert_eq!(
        expr.named_args[1],
        ("per_page".to_string(), serde_json::json!(5))
    );
}

#[test]
fn key_value_args_produce_no_positional_args() {
    let params = search_params();
    let expr = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "repo:rust-lang/rust", "--per_page", "5"]),
        &params,
    )
    .unwrap();

    assert!(expr.positional_args.is_empty());
}

#[test]
fn parses_boolean_flag_without_value() {
    let params = search_params();
    let expr = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "test", "--verbose"]),
        &params,
    )
    .unwrap();

    assert_eq!(expr.named_args[1], ("verbose".to_string(), serde_json::json!(true)));
}

#[test]
fn parses_boolean_flag_with_explicit_false() {
    let params = search_params();
    let expr = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "test", "--verbose", "false"]),
        &params,
    )
    .unwrap();

    assert_eq!(expr.named_args[1], ("verbose".to_string(), serde_json::json!(false)));
}

#[test]
fn parses_json_object_param() {
    let params = vec![ParamSpec {
        name: "meta".to_string(),
        r#type: ParamType::Object,
        required: true,
        default: None,
        description: None,
    }];

    let expr = parse_cli_args("some.cmd", &args(&["--meta", r#"{"key":"val"}"#]), &params).unwrap();

    assert_eq!(expr.named_args[0].1, serde_json::json!({"key": "val"}));
}

#[test]
fn parses_json_array_param() {
    let params = vec![ParamSpec {
        name: "tags".to_string(),
        r#type: ParamType::Array,
        required: true,
        default: None,
        description: None,
    }];

    let expr = parse_cli_args("some.cmd", &args(&["--tags", "[1,2,3]"]), &params).unwrap();

    assert_eq!(expr.named_args[0].1, serde_json::json!([1, 2, 3]));
}

#[test]
#[allow(clippy::approx_constant)] // 3.14 is used as arbitrary user input, not as PI
fn parses_number_param() {
    let params = vec![ParamSpec {
        name: "ratio".to_string(),
        r#type: ParamType::Number,
        required: true,
        default: None,
        description: None,
    }];

    let expr = parse_cli_args("some.cmd", &args(&["--ratio", "3.14"]), &params).unwrap();

    assert_eq!(expr.named_args[0].1.as_f64().unwrap(), 3.14);
}

#[test]
fn error_on_unknown_param() {
    let params = search_params();
    let err = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "test", "--unknown", "x"]),
        &params,
    )
    .unwrap_err();

    assert!(matches!(err, CliArgsError::UnknownParam(..)));
}

#[test]
fn error_on_invalid_integer() {
    let params = search_params();
    let err = parse_cli_args(
        "github.search_issues",
        &args(&["--query", "test", "--per_page", "abc"]),
        &params,
    )
    .unwrap_err();

    assert!(matches!(err, CliArgsError::InvalidValue { .. }));
}

#[test]
fn error_on_missing_value() {
    let params = search_params();
    let err = parse_cli_args("github.search_issues", &args(&["--query"]), &params).unwrap_err();

    assert!(matches!(err, CliArgsError::MissingValue(..)));
}

#[test]
fn error_on_invalid_command_format() {
    let err = parse_cli_args("noperiod", &[], &[]).unwrap_err();
    assert!(matches!(err, CliArgsError::InvalidCommand(..)));
}

#[test]
fn error_on_bare_argument() {
    let params = search_params();
    let err = parse_cli_args("github.search_issues", &args(&["test"]), &params).unwrap_err();
    assert!(matches!(err, CliArgsError::BareArgument(..)));
}

#[test]
fn dot_notation_splits_into_provider_and_command() {
    let expr = parse_cli_args("system.disk_usage", &[], &[]).unwrap();
    assert_eq!(expr.provider, "system");
    assert_eq!(expr.command, "disk_usage");
}
