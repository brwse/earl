use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use earl_core::schema::{AuthTemplate, BodyTemplate, TransportTemplate};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpOperationTemplate {
    pub method: String,
    pub url: String,
    pub path: Option<String>,
    pub query: Option<BTreeMap<String, Value>>,
    pub headers: Option<BTreeMap<String, Value>>,
    pub cookies: Option<BTreeMap<String, Value>>,
    pub auth: Option<AuthTemplate>,
    pub body: Option<BodyTemplate>,
    #[serde(default)]
    pub stream: bool,
    pub transport: Option<TransportTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlOperationTemplate {
    #[serde(default)]
    pub method: String,
    pub url: String,
    pub path: Option<String>,
    pub query: Option<BTreeMap<String, Value>>,
    pub headers: Option<BTreeMap<String, Value>>,
    pub cookies: Option<BTreeMap<String, Value>>,
    pub auth: Option<AuthTemplate>,
    pub graphql: GraphqlTemplate,
    #[serde(default)]
    pub stream: bool,
    pub transport: Option<TransportTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlTemplate {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_operation_defaults_stream_false() {
        let json = r#"{"method":"GET","url":"https://example.com"}"#;
        let op: HttpOperationTemplate = serde_json::from_str(json).unwrap();
        assert!(!op.stream);
    }

    #[test]
    fn http_operation_accepts_stream_true() {
        let json = r#"{"method":"GET","url":"https://example.com","stream":true}"#;
        let op: HttpOperationTemplate = serde_json::from_str(json).unwrap();
        assert!(op.stream);
    }

    #[test]
    fn graphql_operation_defaults_stream_false() {
        let json = r#"{"url":"https://example.com","graphql":{"query":"{ users { id } }"}}"#;
        let op: GraphqlOperationTemplate = serde_json::from_str(json).unwrap();
        assert!(!op.stream);
    }

    #[test]
    fn graphql_operation_accepts_stream_true() {
        let json =
            r#"{"url":"https://example.com","stream":true,"graphql":{"query":"{ users { id } }"}}"#;
        let op: GraphqlOperationTemplate = serde_json::from_str(json).unwrap();
        assert!(op.stream);
    }
}
