use std::collections::BTreeMap;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::template::schema::{ParamSpec, ParamType};

use super::ast::CallExpression;

#[derive(Debug, Error)]
pub enum BindError {
    #[error("too many positional arguments: expected at most {expected}, got {actual}")]
    TooManyPositional { expected: usize, actual: usize },
    #[error("unknown argument `{0}`")]
    UnknownArgument(String),
    #[error("argument `{0}` provided multiple times")]
    DuplicateArgument(String),
    #[error("missing required argument `{0}`")]
    MissingRequired(String),
    #[error("argument `{name}` has invalid type; expected {expected}, got {actual}")]
    InvalidType {
        name: String,
        expected: String,
        actual: String,
    },
}

pub fn bind_arguments(
    expression: &CallExpression,
    params: &[ParamSpec],
) -> Result<Map<String, Value>, BindError> {
    let mut out: BTreeMap<String, Value> = BTreeMap::new();

    if expression.positional_args.len() > params.len() {
        return Err(BindError::TooManyPositional {
            expected: params.len(),
            actual: expression.positional_args.len(),
        });
    }

    for (idx, value) in expression.positional_args.iter().enumerate() {
        let name = params[idx].name.clone();
        if out.insert(name.clone(), value.clone()).is_some() {
            return Err(BindError::DuplicateArgument(name));
        }
    }

    for (name, value) in &expression.named_args {
        if !params.iter().any(|p| p.name == *name) {
            return Err(BindError::UnknownArgument(name.clone()));
        }
        if out.insert(name.clone(), value.clone()).is_some() {
            return Err(BindError::DuplicateArgument(name.clone()));
        }
    }

    for param in params {
        if !out.contains_key(&param.name) {
            if let Some(default_value) = &param.default {
                out.insert(param.name.clone(), default_value.clone());
            } else if param.required {
                return Err(BindError::MissingRequired(param.name.clone()));
            } // else: optional with no default — leave absent; Chainable rendering
              // returns Undefined which maps to null in render_string_value
        }
    }

    for param in params {
        if let Some(value) = out.get(&param.name) {
            // Null means the optional param was not provided — no type to validate.
            if !value.is_null() && !matches_type(value, param.r#type) {
                return Err(BindError::InvalidType {
                    name: param.name.clone(),
                    expected: param.r#type.to_string(),
                    actual: value_type_name(value),
                });
            }
        }
    }

    Ok(out.into_iter().collect())
}

fn matches_type(value: &Value, expected: ParamType) -> bool {
    match expected {
        ParamType::String => value.is_string(),
        ParamType::Integer => value.as_i64().is_some(),
        ParamType::Number => value.is_number(),
        ParamType::Boolean => value.is_boolean(),
        ParamType::Null => value.is_null(),
        ParamType::Array => value.is_array(),
        ParamType::Object => value.is_object(),
    }
}

fn value_type_name(value: &Value) -> String {
    if value.is_string() {
        "string"
    } else if value.as_i64().is_some() {
        "integer"
    } else if value.is_number() {
        "number"
    } else if value.is_boolean() {
        "boolean"
    } else if value.is_null() {
        "null"
    } else if value.is_array() {
        "array"
    } else if value.is_object() {
        "object"
    } else {
        "unknown"
    }
    .to_string()
}
