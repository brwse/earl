use std::collections::BTreeMap;
use std::future::Future;

#[allow(unused_imports)]
use anyhow::{Result, bail};
use base64::Engine;
use serde_json::{Map, Value};

use crate::auth::oauth2::OAuthManager;
use crate::config::{ProxyProfile, SandboxConfig};
use crate::secrets::SecretManager;
use crate::secrets::store::require_secret;
use crate::template::catalog::TemplateCatalogEntry;
use crate::template::render::{render_json_value, render_string_raw};
#[allow(unused_imports)]
use crate::template::schema::{
    AllowRule, ApiKeyLocation, AuthTemplate, CommandMode, OperationTemplate,
};
use earl_core::Redactor;

use super::transport::{ResolvedTransport, resolve_transport};

// ── JinjaRenderer ────────────────────────────────────────────

/// `TemplateRenderer` backed by the main crate's minijinja functions.
struct JinjaRenderer;

impl earl_core::TemplateRenderer for JinjaRenderer {
    fn render_str(&self, template: &str, context: &Value) -> Result<String> {
        render_string_raw(template, context)
    }

    fn render_value(&self, value: &Value, context: &Value) -> Result<Value> {
        render_json_value(value, context)
    }
}

// ── Prepared types ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PreparedRequest {
    pub key: String,
    pub mode: CommandMode,
    pub stream: bool,
    pub allow_rules: Vec<crate::template::schema::AllowRule>,
    pub transport: ResolvedTransport,
    pub result_template: crate::template::schema::ResultTemplate,
    pub args: Map<String, Value>,
    pub redactor: Redactor,
    pub protocol_data: PreparedProtocolData,
}

#[derive(Debug, Clone)]
pub enum PreparedProtocolData {
    #[cfg(feature = "http")]
    Http(PreparedHttpData),
    #[cfg(feature = "graphql")]
    Graphql(PreparedHttpData),
    #[cfg(feature = "grpc")]
    Grpc(PreparedGrpcData),
    #[cfg(feature = "bash")]
    Bash(PreparedBashScript),
    #[cfg(feature = "sql")]
    Sql(PreparedSqlQuery),
}

pub use earl_core::{PreparedBody, PreparedMultipartPart};
#[cfg(feature = "http")]
pub use earl_protocol_http::PreparedHttpData;

#[cfg(feature = "grpc")]
pub use earl_protocol_grpc::PreparedGrpcData;

#[cfg(feature = "bash")]
pub use earl_protocol_bash::PreparedBashScript;

#[cfg(feature = "sql")]
pub use earl_protocol_sql::PreparedSqlQuery;

// ── Builder entry-points ─────────────────────────────────────

pub async fn build_prepared_request(
    entry: &TemplateCatalogEntry,
    args: Map<String, Value>,
    secret_manager: &SecretManager,
    oauth_manager: &OAuthManager,
    allow_rules: &[AllowRule],
    proxy_profiles: &BTreeMap<String, ProxyProfile>,
    sandbox_config: &SandboxConfig,
) -> Result<PreparedRequest> {
    build_prepared_request_with_token_provider(
        entry,
        args,
        secret_manager,
        |profile| async move { oauth_manager.access_token_for_profile(&profile).await },
        allow_rules,
        proxy_profiles,
        sandbox_config,
    )
    .await
}

pub async fn build_prepared_request_with_token_provider<F, Fut>(
    entry: &TemplateCatalogEntry,
    args: Map<String, Value>,
    secret_manager: &SecretManager,
    mut oauth_token_provider: F,
    allow_rules: &[AllowRule],
    proxy_profiles: &BTreeMap<String, ProxyProfile>,
    sandbox_config: &SandboxConfig,
) -> Result<PreparedRequest>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let mut secret_values = Vec::new();
    let mut secrets_context = Map::new();

    for secret_key in &entry.template.annotations.secrets {
        let secret = require_secret(secret_manager.store(), secret_key)?;
        insert_dotted_key(
            &mut secrets_context,
            secret_key,
            Value::String(secret.clone()),
        );
        secret_values.push(secret);
    }

    let context = Value::Object(Map::from_iter([
        ("args".to_string(), Value::Object(args.clone())),
        (
            "secrets".to_string(),
            Value::Object(secrets_context.clone()),
        ),
    ]));

    let renderer = JinjaRenderer;
    let operation = &entry.template.operation;
    let transport = resolve_transport(operation.transport(), proxy_profiles)?;

    #[allow(unreachable_patterns)]
    match operation {
        #[cfg(feature = "http")]
        OperationTemplate::Http(http) => {
            let mut data = earl_protocol_http::builder::build_http_request(
                http, &context, &renderer, &entry.key,
            )?;

            if let Some(auth) = &http.auth {
                let mut outputs = AuthOutputs {
                    headers: &mut data.headers,
                    query: &mut data.query,
                    cookies: &mut data.cookies,
                    secret_values: &mut secret_values,
                };
                apply_auth_inner(
                    auth,
                    &context,
                    secret_manager,
                    &mut oauth_token_provider,
                    &mut outputs,
                )
                .await?;
            }

            Ok(PreparedRequest {
                key: entry.key.clone(),
                mode: entry.mode,
                stream: entry.template.operation.is_streaming(),
                allow_rules: allow_rules.to_vec(),
                transport,
                result_template: entry.template.result.clone(),
                args,
                redactor: Redactor::new(secret_values),
                protocol_data: PreparedProtocolData::Http(data),
            })
        }
        #[cfg(feature = "graphql")]
        OperationTemplate::Graphql(graphql) => {
            let mut data = earl_protocol_http::builder::build_graphql_request(
                graphql, &context, &renderer, &entry.key,
            )?;

            if let Some(auth) = &graphql.auth {
                let mut outputs = AuthOutputs {
                    headers: &mut data.headers,
                    query: &mut data.query,
                    cookies: &mut data.cookies,
                    secret_values: &mut secret_values,
                };
                apply_auth_inner(
                    auth,
                    &context,
                    secret_manager,
                    &mut oauth_token_provider,
                    &mut outputs,
                )
                .await?;
            }

            Ok(PreparedRequest {
                key: entry.key.clone(),
                mode: entry.mode,
                stream: entry.template.operation.is_streaming(),
                allow_rules: allow_rules.to_vec(),
                transport,
                result_template: entry.template.result.clone(),
                args,
                redactor: Redactor::new(secret_values),
                protocol_data: PreparedProtocolData::Graphql(data),
            })
        }
        #[cfg(feature = "grpc")]
        OperationTemplate::Grpc(grpc_operation) => {
            let mut data = earl_protocol_grpc::builder::build_grpc_request(
                grpc_operation,
                &context,
                &renderer,
                &entry.key,
            )?;

            let mut query = Vec::new();
            let mut cookies = Vec::new();

            if let Some(auth) = &grpc_operation.auth {
                let mut outputs = AuthOutputs {
                    headers: &mut data.headers,
                    query: &mut query,
                    cookies: &mut cookies,
                    secret_values: &mut secret_values,
                };
                apply_auth_inner(
                    auth,
                    &context,
                    secret_manager,
                    &mut oauth_token_provider,
                    &mut outputs,
                )
                .await?;
            }

            if !query.is_empty() || !cookies.is_empty() {
                bail!(
                    "template `{}` gRPC auth must use header-based credentials (query/cookie auth is unsupported)",
                    entry.key
                );
            }

            Ok(PreparedRequest {
                key: entry.key.clone(),
                mode: entry.mode,
                stream: entry.template.operation.is_streaming(),
                allow_rules: allow_rules.to_vec(),
                transport,
                result_template: entry.template.result.clone(),
                args,
                redactor: Redactor::new(secret_values),
                protocol_data: PreparedProtocolData::Grpc(data),
            })
        }
        #[cfg(feature = "bash")]
        OperationTemplate::Bash(bash_operation) => {
            let global_limits = earl_protocol_bash::builder::GlobalBashLimits {
                allow_network: sandbox_config.bash_allow_network,
                max_time_ms: sandbox_config.bash_max_time_ms,
                max_output_bytes: sandbox_config.bash_max_output_bytes,
            };

            let data = earl_protocol_bash::builder::build_bash_request(
                bash_operation,
                &context,
                &renderer,
                &global_limits,
            )?;

            Ok(PreparedRequest {
                key: entry.key.clone(),
                mode: entry.mode,
                stream: entry.template.operation.is_streaming(),
                allow_rules: Vec::new(),
                transport,
                result_template: entry.template.result.clone(),
                args,
                redactor: Redactor::new(secret_values),
                protocol_data: PreparedProtocolData::Bash(data),
            })
        }
        #[cfg(feature = "sql")]
        OperationTemplate::Sql(sql_operation) => {
            let connection_secret_key = &sql_operation.sql.connection_secret;

            if !sandbox_config.sql_connection_allowlist.is_empty()
                && !sandbox_config
                    .sql_connection_allowlist
                    .iter()
                    .any(|s| s == connection_secret_key)
            {
                bail!(
                    "template `{}` sql connection_secret `{}` is not in sandbox.sql_connection_allowlist",
                    entry.key,
                    connection_secret_key
                );
            }

            let connection_url = require_secret(secret_manager.store(), connection_secret_key)?;
            secret_values.push(connection_url.clone());

            let mut data = earl_protocol_sql::builder::build_sql_request(
                sql_operation,
                &context,
                &renderer,
                connection_url,
            )?;

            // Enforce global sandbox caps (most-restrictive-wins).
            if sandbox_config.sql_force_read_only {
                data.read_only = true;
            }
            if let Some(global_max_rows) = sandbox_config.sql_max_rows {
                let global = global_max_rows as usize;
                data.max_rows = data.max_rows.min(global);
            }

            Ok(PreparedRequest {
                key: entry.key.clone(),
                mode: entry.mode,
                stream: entry.template.operation.is_streaming(),
                allow_rules: Vec::new(),
                transport,
                result_template: entry.template.result.clone(),
                args,
                redactor: Redactor::new(secret_values),
                protocol_data: PreparedProtocolData::Sql(data),
            })
        }
        _ => bail!("unsupported protocol (feature not enabled)"),
    }
}

// ── Auth (stays in main crate) ───────────────────────────────

async fn apply_auth_inner<F, Fut>(
    auth: &AuthTemplate,
    context: &Value,
    secret_manager: &SecretManager,
    oauth_token_provider: &mut F,
    outputs: &mut AuthOutputs<'_>,
) -> Result<()>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    match auth {
        AuthTemplate::None => {}
        AuthTemplate::ApiKey {
            location,
            name,
            secret,
        } => {
            let value = require_secret(secret_manager.store(), secret)?;
            outputs.secret_values.push(value.clone());
            match location {
                ApiKeyLocation::Header => outputs.headers.push((name.clone(), value)),
                ApiKeyLocation::Query => outputs.query.push((name.clone(), value)),
                ApiKeyLocation::Cookie => outputs.cookies.push((name.clone(), value)),
            }
        }
        AuthTemplate::Bearer { secret } => {
            let token = require_secret(secret_manager.store(), secret)?;
            outputs.secret_values.push(token.clone());
            outputs
                .headers
                .push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        AuthTemplate::Basic {
            username,
            password_secret,
        } => {
            let user = render_string_raw(username, context)?;
            let password = require_secret(secret_manager.store(), password_secret)?;
            outputs.secret_values.push(password.clone());
            let encoded =
                base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
            outputs
                .headers
                .push(("Authorization".to_string(), format!("Basic {encoded}")));
        }
        AuthTemplate::OAuth2Profile { profile } => {
            let token = oauth_token_provider(profile.clone()).await?;
            outputs.secret_values.push(token.clone());
            outputs
                .headers
                .push(("Authorization".to_string(), format!("Bearer {token}")));
        }
    }

    Ok(())
}

struct AuthOutputs<'a> {
    headers: &'a mut Vec<(String, String)>,
    query: &'a mut Vec<(String, String)>,
    cookies: &'a mut Vec<(String, String)>,
    secret_values: &'a mut Vec<String>,
}

// ── Helpers ──────────────────────────────────────────────────

fn insert_dotted_key(root: &mut Map<String, Value>, dotted_key: &str, value: Value) {
    let parts: Vec<&str> = dotted_key.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return;
    }

    let mut current = root;
    for (index, part) in parts.iter().enumerate() {
        if index == parts.len() - 1 {
            current.insert((*part).to_string(), value.clone());
            break;
        }

        let child = current
            .entry((*part).to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        if !child.is_object() {
            *child = Value::Object(Map::new());
        }

        current = child.as_object_mut().expect("object ensured above");
    }
}
