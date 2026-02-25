use earl::expression::ast::CallExpression;
use earl::expression::binder::{bind_arguments, BindError};
use earl::template::schema::{ParamSpec, ParamType};

fn params() -> Vec<ParamSpec> {
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
            name: "include".to_string(),
            r#type: ParamType::Boolean,
            required: false,
            default: Some(serde_json::json!(false)),
            description: None,
        },
    ]
}

fn params_with_optional() -> Vec<ParamSpec> {
    let mut p = params();
    p.push(ParamSpec {
        name: "filter".to_string(),
        r#type: ParamType::String,
        required: false,
        default: None,
        description: None,
    });
    p
}

#[test]
fn positional_argument_binds_by_position() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![serde_json::json!("hello")],
        named_args: vec![("per_page".to_string(), serde_json::json!(10))],
    };
    let bound = bind_arguments(&expr, &params()).unwrap();

    assert_eq!(bound.get("query").unwrap(), "hello");
}

#[test]
fn named_argument_binds_by_name() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![serde_json::json!("hello")],
        named_args: vec![("per_page".to_string(), serde_json::json!(10))],
    };
    let bound = bind_arguments(&expr, &params()).unwrap();

    assert_eq!(bound.get("per_page").unwrap(), 10);
}

#[test]
fn optional_param_with_default_is_populated() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![serde_json::json!("hello")],
        named_args: vec![],
    };
    let bound = bind_arguments(&expr, &params()).unwrap();

    assert_eq!(bound.get("include").unwrap(), false);
}

#[test]
fn optional_param_without_default_is_absent() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![serde_json::json!("hello")],
        named_args: vec![],
    };
    let bound = bind_arguments(&expr, &params_with_optional()).unwrap();

    // Optional params with no default are not injected into the map.
    // Chainable rendering converts the resulting Undefined to null.
    assert!(!bound.contains_key("filter"));
}

#[test]
fn fails_on_missing_required_argument() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![],
        named_args: vec![],
    };
    let err = bind_arguments(&expr, &params()).unwrap_err();
    assert!(matches!(err, BindError::MissingRequired(_)));
}

#[test]
fn fails_on_unknown_argument() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![],
        named_args: vec![
            ("query".to_string(), serde_json::json!("x")),
            ("unknown".to_string(), serde_json::json!(1)),
        ],
    };
    let err = bind_arguments(&expr, &params()).unwrap_err();
    assert!(matches!(err, BindError::UnknownArgument(_)));
}

#[test]
fn fails_on_too_many_positional_arguments() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![
            serde_json::json!("a"),
            serde_json::json!(1),
            serde_json::json!(true),
            serde_json::json!("extra"),
        ],
        named_args: vec![],
    };
    let err = bind_arguments(&expr, &params()).unwrap_err();
    assert!(matches!(err, BindError::TooManyPositional { .. }));
}

#[test]
fn fails_on_invalid_argument_type() {
    let expr = CallExpression {
        provider: "github".to_string(),
        command: "search_issues".to_string(),
        positional_args: vec![],
        named_args: vec![
            ("query".to_string(), serde_json::json!("x")),
            ("per_page".to_string(), serde_json::json!("ten")),
        ],
    };
    let err = bind_arguments(&expr, &params()).unwrap_err();
    assert!(matches!(err, BindError::InvalidType { .. }));
}
