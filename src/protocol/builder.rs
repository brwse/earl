use std::collections::BTreeMap;
use std::future::Future;

#[allow(unused_imports)]
use anyhow::{Context, Result, bail};
use base64::Engine;
use serde_json::{Map, Value};

use crate::auth::oauth2::OAuthManager;
use crate::config::{ProxyProfile, SandboxConfig};
use crate::secrets::SecretManager;
use crate::secrets::store::require_secret;
use crate::template::catalog::TemplateCatalogEntry;
use crate::template::environments::select_for_env;
use crate::template::render::{render_json_value, render_string_raw};
#[allow(unused_imports)]
use crate::template::schema::{
    AllowRule, ApiKeyLocation, AuthTemplate, CommandMode, OperationTemplate, ProviderEnvironments,
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

#[allow(clippy::too_many_arguments)]
pub async fn build_prepared_request(
    entry: &TemplateCatalogEntry,
    args: Map<String, Value>,
    secret_manager: &SecretManager,
    oauth_manager: &OAuthManager,
    allow_rules: &[AllowRule],
    proxy_profiles: &BTreeMap<String, ProxyProfile>,
    sandbox_config: &SandboxConfig,
    active_env: Option<&str>,
) -> Result<PreparedRequest> {
    build_prepared_request_with_token_provider(
        entry,
        args,
        secret_manager,
        |profile| async move { oauth_manager.access_token_for_profile(&profile).await },
        allow_rules,
        proxy_profiles,
        sandbox_config,
        active_env,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn build_prepared_request_with_token_provider<F, Fut>(
    entry: &TemplateCatalogEntry,
    args: Map<String, Value>,
    secret_manager: &SecretManager,
    mut oauth_token_provider: F,
    allow_rules: &[AllowRule],
    proxy_profiles: &BTreeMap<String, ProxyProfile>,
    sandbox_config: &SandboxConfig,
    active_env: Option<&str>,
) -> Result<PreparedRequest>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let mut secret_values = Vec::new();
    let mut secrets_context = Map::new();

    for secret_key in &entry.template.annotations.secrets {
        let secret = require_secret(
            secret_manager.store(),
            secret_manager.resolvers(),
            secret_key,
        )?;
        insert_dotted_key(
            &mut secrets_context,
            secret_key,
            Value::String(secret.clone()),
        );
        secret_values.push(secret);
    }

    // Load environment-level secrets declared in environments.secrets
    // (so they're available in the secrets context for vars rendering).
    if let Some(envs) = &entry.provider_environments {
        for secret_key in &envs.secrets {
            if !entry.template.annotations.secrets.contains(secret_key) {
                let secret = require_secret(
                    secret_manager.store(),
                    secret_manager.resolvers(),
                    secret_key,
                )?;
                insert_dotted_key(
                    &mut secrets_context,
                    secret_key,
                    Value::String(secret.clone()),
                );
                secret_values.push(secret);
            }
        }
    }

    // Resolve vars for the active environment.
    let vars_context = resolve_vars(
        entry.provider_environments.as_ref(),
        active_env,
        &Value::Object(secrets_context.clone()),
        &mut secret_values,
    )?;

    let context = Value::Object(Map::from_iter([
        ("args".to_string(), Value::Object(args.clone())),
        (
            "secrets".to_string(),
            Value::Object(secrets_context.clone()),
        ),
        ("vars".to_string(), Value::Object(vars_context)),
    ]));

    let renderer = JinjaRenderer;
    let (operation, result_template) = select_for_env(&entry.template, active_env);
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
                stream: operation.is_streaming(),
                allow_rules: allow_rules.to_vec(),
                transport,
                result_template: result_template.clone(),
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
                stream: operation.is_streaming(),
                allow_rules: allow_rules.to_vec(),
                transport,
                result_template: result_template.clone(),
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
                stream: operation.is_streaming(),
                allow_rules: allow_rules.to_vec(),
                transport,
                result_template: result_template.clone(),
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
                max_memory_bytes: sandbox_config.bash_max_memory_bytes,
                max_cpu_time_ms: sandbox_config.bash_max_cpu_time_ms,
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
                stream: operation.is_streaming(),
                allow_rules: Vec::new(),
                transport,
                result_template: result_template.clone(),
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

            let connection_url = require_secret(
                secret_manager.store(),
                secret_manager.resolvers(),
                connection_secret_key,
            )?;
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
                stream: operation.is_streaming(),
                allow_rules: Vec::new(),
                transport,
                result_template: result_template.clone(),
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
            let value = require_secret(secret_manager.store(), secret_manager.resolvers(), secret)?;
            outputs.secret_values.push(value.clone());
            match location {
                ApiKeyLocation::Header => outputs.headers.push((name.clone(), value)),
                ApiKeyLocation::Query => outputs.query.push((name.clone(), value)),
                ApiKeyLocation::Cookie => outputs.cookies.push((name.clone(), value)),
            }
        }
        AuthTemplate::Bearer { secret } => {
            let token = require_secret(secret_manager.store(), secret_manager.resolvers(), secret)?;
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
            let password = require_secret(
                secret_manager.store(),
                secret_manager.resolvers(),
                password_secret,
            )?;
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

/// Resolves the active environment's variable set.
///
/// Each variable value is rendered as a Jinja template with only `secrets`
/// available in the render context — `args` are not available here.
/// Every resolved value is added to `secret_values` for redaction (since
/// values may be derived from secrets).
///
/// Returns an empty map if there is no active environment or no environments
/// block is configured.
fn resolve_vars(
    provider_envs: Option<&ProviderEnvironments>,
    env_name: Option<&str>,
    secrets_context: &Value,
    secret_values: &mut Vec<String>,
) -> Result<Map<String, Value>> {
    let Some(envs) = provider_envs else {
        return Ok(Map::new());
    };
    let Some(name) = env_name else {
        return Ok(Map::new());
    };
    let env_vars = match envs.environments.get(name) {
        Some(v) => v,
        None => bail!(
            "environment `{name}` is not defined; available: {}",
            envs.environments
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };

    let render_ctx = Value::Object(Map::from_iter([(
        "secrets".to_string(),
        secrets_context.clone(),
    )]));

    let mut resolved = Map::new();
    for (key, template_str) in env_vars {
        let rendered = render_string_raw(template_str, &render_ctx)
            .with_context(|| format!("failed rendering vars.{key} for environment `{name}`"))?;
        // Track every rendered value for redaction. Even plain-string vars (e.g.
        // `base_url`) are redacted because they may contain secret-derived content
        // and we can't cheaply distinguish them from pure constants at this stage.
        // This means non-secret vars will also be redacted from output and error
        // messages; that's the chosen tradeoff for defence-in-depth.
        secret_values.push(rendered.clone());
        resolved.insert(key.clone(), Value::String(rendered));
    }
    Ok(resolved)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::schema::ProviderEnvironments;
    use std::collections::BTreeMap;

    #[test]
    fn no_environments_block_returns_empty_map() {
        let mut secret_values = vec![];
        let secrets = Value::Object(Map::new());
        let result = resolve_vars(None, None, &secrets, &mut secret_values).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn no_environments_block_tracks_no_secrets() {
        let mut secret_values = vec![];
        let secrets = Value::Object(Map::new());
        resolve_vars(None, None, &secrets, &mut secret_values).unwrap();
        assert!(secret_values.is_empty());
    }

    #[test]
    fn no_active_environment_returns_empty_map() {
        let mut staging_vars = BTreeMap::new();
        staging_vars.insert(
            "base_url".to_string(),
            "https://staging.example.com".to_string(),
        );
        let pe = ProviderEnvironments {
            default: None,
            secrets: vec![],
            environments: BTreeMap::from([("staging".to_string(), staging_vars)]),
        };
        let mut secret_values = vec![];
        let secrets = Value::Object(Map::new());
        let result = resolve_vars(Some(&pe), None, &secrets, &mut secret_values).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn resolved_var_has_correct_value() {
        let mut staging_vars = BTreeMap::new();
        staging_vars.insert("label".to_string(), "staging-label".to_string());
        let pe = ProviderEnvironments {
            default: None,
            secrets: vec![],
            environments: BTreeMap::from([("staging".to_string(), staging_vars)]),
        };
        let mut secret_values = vec![];
        let secrets = Value::Object(Map::new());
        let result =
            resolve_vars(Some(&pe), Some("staging"), &secrets, &mut secret_values).unwrap();
        assert_eq!(result["label"], Value::String("staging-label".to_string()));
    }

    #[test]
    fn resolved_var_is_tracked_for_redaction() {
        let mut staging_vars = BTreeMap::new();
        staging_vars.insert("label".to_string(), "staging-label".to_string());
        let pe = ProviderEnvironments {
            default: None,
            secrets: vec![],
            environments: BTreeMap::from([("staging".to_string(), staging_vars)]),
        };
        let mut secret_values = vec![];
        let secrets = Value::Object(Map::new());
        resolve_vars(Some(&pe), Some("staging"), &secrets, &mut secret_values).unwrap();
        assert!(secret_values.contains(&"staging-label".to_string()));
    }

    #[test]
    fn unknown_environment_name_returns_error() {
        let pe = ProviderEnvironments {
            default: None,
            secrets: vec![],
            environments: BTreeMap::from([("staging".to_string(), BTreeMap::new())]),
        };
        let mut secret_values = vec![];
        let secrets = Value::Object(Map::new());
        resolve_vars(Some(&pe), Some("ghost"), &secrets, &mut secret_values).unwrap_err();
    }
}
