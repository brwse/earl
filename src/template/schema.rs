use std::collections::BTreeMap;

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
    pub commands: BTreeMap<String, CommandTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Annotations {
    #[serde(default)]
    pub mode: CommandMode,
    #[serde(default)]
    pub secrets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
