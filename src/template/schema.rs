use std::collections::BTreeMap;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

pub use earl_core::schema::{
    AllowRule, ApiKeyLocation, AuthTemplate, BodyTemplate, CommandMode, MultipartPartTemplate,
    ParamSpec, ParamType, RedirectTemplate, ResultDecode, ResultExtract, ResultTemplate,
    RetryTemplate, TlsTemplate, TransportTemplate,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateFile {
    pub version: u32,
    pub provider: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub environments: Option<ProviderEnvironments>,
    pub commands: BTreeMap<String, CommandTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandTemplate {
    pub title: String,
    pub summary: String,
    pub description: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub annotations: Annotations,
    #[serde(default)]
    pub params: Vec<ParamSpec>,
    pub operation: OperationTemplate,
    #[serde(default)]
    pub result: ResultTemplate,
    #[serde(default)]
    pub environment_overrides: BTreeMap<String, EnvironmentOverride>,
}

#[derive(
    Debug, Clone, Default, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(deny_unknown_fields)]
pub struct Annotations {
    #[serde(default)]
    pub mode: CommandMode,
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub allow_environment_protocol_switching: bool,
}

/// Provider-level environments block stored at the TemplateFile level.
/// Carried into TemplateCatalogEntry so it's available at call time.
#[derive(
    Debug, Clone, Default, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[serde(deny_unknown_fields)]
pub struct ProviderEnvironments {
    #[serde(default)]
    pub default: Option<String>,
    /// Secrets that must be loaded before rendering vars values.
    #[serde(default)]
    pub secrets: Vec<String>,
    /// Named environments, each mapping variable name → Jinja template string.
    #[serde(default)]
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
}

/// Per-command environment override: when active env matches, fully replaces
/// the command's default operation (and optionally result).
#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentOverride {
    pub operation: OperationTemplate,
    #[serde(default)]
    pub result: Option<ResultTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(tag = "protocol", rename_all = "snake_case")]
pub enum OperationTemplate {
    #[cfg(feature = "http")]
    Http(HttpOperationTemplate),
    #[cfg(feature = "graphql")]
    Graphql(GraphqlOperationTemplate),
    #[cfg(feature = "grpc")]
    Grpc(GrpcOperationTemplate),
    #[cfg(feature = "bash")]
    Bash(BashOperationTemplate),
    #[cfg(feature = "sql")]
    Sql(SqlOperationTemplate),
}

impl OperationTemplate {
    #[allow(unreachable_patterns)]
    pub fn protocol(&self) -> OperationProtocol {
        match self {
            #[cfg(feature = "http")]
            OperationTemplate::Http(_) => OperationProtocol::Http,
            #[cfg(feature = "graphql")]
            OperationTemplate::Graphql(_) => OperationProtocol::Graphql,
            #[cfg(feature = "grpc")]
            OperationTemplate::Grpc(_) => OperationProtocol::Grpc,
            #[cfg(feature = "bash")]
            OperationTemplate::Bash(_) => OperationProtocol::Bash,
            #[cfg(feature = "sql")]
            OperationTemplate::Sql(_) => OperationProtocol::Sql,
            _ => unreachable!(),
        }
    }

    #[allow(unreachable_patterns)]
    pub fn transport(&self) -> Option<&TransportTemplate> {
        match self {
            #[cfg(feature = "http")]
            OperationTemplate::Http(op) => op.transport.as_ref(),
            #[cfg(feature = "graphql")]
            OperationTemplate::Graphql(op) => op.transport.as_ref(),
            #[cfg(feature = "grpc")]
            OperationTemplate::Grpc(op) => op.transport.as_ref(),
            #[cfg(feature = "bash")]
            OperationTemplate::Bash(op) => op.transport.as_ref(),
            #[cfg(feature = "sql")]
            OperationTemplate::Sql(op) => op.transport.as_ref(),
            _ => None,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn auth(&self) -> Option<&AuthTemplate> {
        match self {
            #[cfg(feature = "http")]
            OperationTemplate::Http(op) => op.auth.as_ref(),
            #[cfg(feature = "graphql")]
            OperationTemplate::Graphql(op) => op.auth.as_ref(),
            #[cfg(feature = "grpc")]
            OperationTemplate::Grpc(op) => op.auth.as_ref(),
            _ => None,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn request_url(&self) -> Option<&str> {
        match self {
            #[cfg(feature = "http")]
            OperationTemplate::Http(op) => Some(op.url.as_str()),
            #[cfg(feature = "graphql")]
            OperationTemplate::Graphql(op) => Some(op.url.as_str()),
            #[cfg(feature = "grpc")]
            OperationTemplate::Grpc(op) => Some(op.url.as_str()),
            _ => None,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn grpc_service_method(&self) -> Option<(&str, &str)> {
        match self {
            #[cfg(feature = "grpc")]
            OperationTemplate::Grpc(op) => {
                Some((op.grpc.service.as_str(), op.grpc.method.as_str()))
            }
            _ => None,
        }
    }

    #[cfg(feature = "bash")]
    pub fn bash_script(&self) -> Option<&str> {
        match self {
            OperationTemplate::Bash(op) => Some(op.bash.script.as_str()),
            _ => None,
        }
    }

    #[cfg(feature = "sql")]
    pub fn sql_query(&self) -> Option<&str> {
        match self {
            OperationTemplate::Sql(op) => Some(op.sql.query.as_str()),
            _ => None,
        }
    }

    #[allow(unreachable_patterns)]
    pub fn is_streaming(&self) -> bool {
        match self {
            #[cfg(feature = "http")]
            OperationTemplate::Http(op) => op.stream,
            #[cfg(feature = "graphql")]
            OperationTemplate::Graphql(op) => op.stream,
            #[cfg(feature = "grpc")]
            OperationTemplate::Grpc(op) => op.stream,
            #[cfg(feature = "bash")]
            OperationTemplate::Bash(op) => op.stream,
            #[cfg(feature = "sql")]
            OperationTemplate::Sql(_) => false,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationProtocol {
    #[cfg(feature = "http")]
    Http,
    #[cfg(feature = "graphql")]
    Graphql,
    #[cfg(feature = "grpc")]
    Grpc,
    #[cfg(feature = "bash")]
    Bash,
    #[cfg(feature = "sql")]
    Sql,
}

#[cfg(feature = "http")]
pub use earl_protocol_http::HttpOperationTemplate;
#[cfg(feature = "graphql")]
pub use earl_protocol_http::{GraphqlOperationTemplate, GraphqlTemplate};

#[cfg(feature = "grpc")]
pub use earl_protocol_grpc::{GrpcOperationTemplate, GrpcTemplate};

// ── Bash ──────────────────────────────────────────────

#[cfg(feature = "bash")]
pub use earl_protocol_bash::{BashOperationTemplate, BashSandboxTemplate, BashScriptTemplate};

// ── SQL ───────────────────────────────────────────────

#[cfg(feature = "sql")]
pub use earl_protocol_sql::{SqlOperationTemplate, SqlQueryTemplate, SqlSandboxTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_environments_deserializes_from_normalized_json() {
        let json = serde_json::json!({
            "default": "production",
            "secrets": ["myservice.prod_token"],
            "environments": {
                "production": { "base_url": "https://api.myservice.com" },
                "staging":    { "base_url": "https://staging.myservice.com" }
            }
        });
        let pe: ProviderEnvironments = serde_json::from_value(json).unwrap();
        assert_eq!(pe.default.as_deref(), Some("production"));
        assert_eq!(pe.secrets, vec!["myservice.prod_token"]);
        assert_eq!(
            pe.environments["production"]["base_url"],
            "https://api.myservice.com"
        );
        assert_eq!(
            pe.environments["staging"]["base_url"],
            "https://staging.myservice.com"
        );
    }

    #[test]
    fn provider_environments_defaults_work() {
        let json = serde_json::json!({
            "environments": { "staging": { "url": "https://staging.example.com" } }
        });
        let pe: ProviderEnvironments = serde_json::from_value(json).unwrap();
        assert!(pe.default.is_none());
        assert!(pe.secrets.is_empty());
        assert!(pe.environments.contains_key("staging"));
    }
}
