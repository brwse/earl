use std::collections::HashSet;
#[cfg(feature = "bash")]
use std::path::{Component, Path};

use anyhow::{Result, bail};

use super::environments::validate_env_name;
#[cfg(feature = "bash")]
use super::schema::BashOperationTemplate;
#[cfg(feature = "graphql")]
use super::schema::GraphqlOperationTemplate;
#[cfg(feature = "grpc")]
use super::schema::GrpcOperationTemplate;
#[cfg(feature = "http")]
use super::schema::HttpOperationTemplate;
#[cfg(feature = "sql")]
use super::schema::SqlOperationTemplate;
#[allow(unused_imports)]
use super::schema::{
    ApiKeyLocation, AuthTemplate, BodyTemplate, CommandTemplate, MultipartPartTemplate,
    OperationTemplate, ParamSpec, TemplateFile, TransportTemplate,
};

pub fn validate_template_file(file: &TemplateFile) -> Result<()> {
    if file.version != 1 {
        bail!(
            "unsupported template version {} for provider {}",
            file.version,
            file.provider
        );
    }
    if file.provider.trim().is_empty() {
        bail!("template provider must not be empty");
    }
    if file.commands.is_empty() {
        bail!("provider {} defines no commands", file.provider);
    }

    // Build set of defined environment names for cross-reference checks
    let defined_env_names: std::collections::HashSet<String> = file
        .environments
        .as_ref()
        .map(|e| e.environments.keys().cloned().collect())
        .unwrap_or_default();

    if let Some(envs) = &file.environments {
        // environments.default must reference a defined environment
        if let Some(default_name) = &envs.default {
            validate_env_name(default_name).map_err(|e| {
                anyhow::anyhow!("provider `{}` environments.default: {e}", file.provider)
            })?;
            if !envs.environments.contains_key(default_name.as_str()) {
                bail!(
                    "provider `{}` environments.default is `{default_name}` but that environment is not defined; \
                     available: {}",
                    file.provider,
                    envs.environments
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
        // Validate format of all declared environment names
        for env_key in envs.environments.keys() {
            validate_env_name(env_key).map_err(|e| {
                anyhow::anyhow!(
                    "provider `{}` environments block contains invalid name `{env_key}`: {e}",
                    file.provider
                )
            })?;
        }
        // All secrets referenced in vars values must be declared in environments.secrets
        let declared_secrets: std::collections::HashSet<&str> =
            envs.secrets.iter().map(String::as_str).collect();
        for (env_name, vars) in &envs.environments {
            for (var_name, value) in vars {
                for secret_ref in extract_secret_refs(value) {
                    if !declared_secrets.contains(secret_ref) {
                        bail!(
                            "provider `{}` environments.{env_name}.{var_name} references secret \
                             `{secret_ref}` which is not declared in environments.secrets",
                            file.provider
                        );
                    }
                }
            }
        }
    }

    for (name, cmd) in &file.commands {
        for (env_name, override_) in &cmd.environment_overrides {
            // Environment override names must follow the same format rules as CLI --env values
            validate_env_name(env_name).map_err(|e| {
                anyhow::anyhow!(
                    "command `{name}` has invalid environment override name `{env_name}`: {e}"
                )
            })?;
            // Per-command environment names must be defined in the provider environments block
            // when one exists. When there is no provider-level environments block the cross-
            // reference check is skipped: per-command overrides are valid without a global
            // block (they activate via `--env <name>` and `vars.*` will be empty). Template
            // authors relying only on operation overrides without vars injection may omit the
            // global block intentionally.
            if !defined_env_names.is_empty() && !defined_env_names.contains(env_name) {
                bail!(
                    "command `{name}` has environment override for `{env_name}` \
                     which is not defined in the provider environments block; \
                     defined: {}",
                    defined_env_names
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            // Protocol switching requires annotation
            if override_.operation.protocol() != cmd.operation.protocol()
                && !cmd.annotations.allow_environment_protocol_switching
            {
                bail!(
                    "command `{name}` environment `{env_name}` switches protocol \
                     from {:?} to {:?}; add `annotations {{ allow_environment_protocol_switching = true }}` \
                     to opt in",
                    cmd.operation.protocol(),
                    override_.operation.protocol()
                );
            }
            // Validate the override operation itself
            validate_operation(name, &override_.operation, &cmd.annotations.secrets)?;
            // Validate override result if provided
            if let Some(result) = &override_.result
                && result.output.trim().is_empty()
            {
                bail!("command `{name}` environment override `{env_name}` has empty result.output");
            }
        }
        validate_command(name, cmd)?;
    }

    Ok(())
}

/// Extracts `secrets.X.Y` references from Jinja `{{ secrets.X.Y }}` expressions.
fn extract_secret_refs(value: &str) -> Vec<&str> {
    let mut refs = Vec::new();
    let mut remaining = value;
    while let Some(start) = remaining.find("{{") {
        remaining = &remaining[start + 2..];
        let end = match remaining.find("}}") {
            Some(e) => e,
            None => break,
        };
        let expr = remaining[..end].trim();
        if let Some(key) = expr.strip_prefix("secrets.") {
            // Take everything up to the first whitespace or pipe
            let key = key
                .split(|c: char| c.is_whitespace() || c == '|')
                .next()
                .unwrap_or(key);
            let key = key.trim_end_matches('.');
            refs.push(key);
        }
        remaining = &remaining[end + 2..];
    }
    refs
}

fn validate_command(command_name: &str, cmd: &CommandTemplate) -> Result<()> {
    if cmd.title.trim().is_empty() {
        bail!("command {command_name} has empty title");
    }

    if cmd.summary.trim().is_empty() {
        bail!("command {command_name} has empty summary");
    }

    if cmd.description.trim().is_empty() {
        bail!("command {command_name} has empty description");
    }

    validate_operation(command_name, &cmd.operation, &cmd.annotations.secrets)?;

    if cmd.result.output.trim().is_empty() {
        bail!("command {command_name} has empty result.output");
    }

    validate_params(command_name, &cmd.params)?;

    Ok(())
}

fn validate_params(command_name: &str, params: &[ParamSpec]) -> Result<()> {
    let mut seen = HashSet::new();
    for param in params {
        if param.name.trim().is_empty() {
            bail!("command {command_name} has parameter with empty name");
        }
        if !seen.insert(&param.name) {
            bail!(
                "command {command_name} has duplicate parameter `{}`",
                param.name
            );
        }
    }
    Ok(())
}

fn validate_operation(
    command_name: &str,
    operation: &OperationTemplate,
    allowed_secrets: &[String],
) -> Result<()> {
    #[allow(unreachable_patterns)]
    match operation {
        #[cfg(feature = "http")]
        OperationTemplate::Http(op) => validate_http_operation(command_name, op, allowed_secrets),
        #[cfg(feature = "graphql")]
        OperationTemplate::Graphql(op) => {
            validate_graphql_operation(command_name, op, allowed_secrets)
        }
        #[cfg(feature = "grpc")]
        OperationTemplate::Grpc(op) => validate_grpc_operation(command_name, op, allowed_secrets),
        #[cfg(feature = "bash")]
        OperationTemplate::Bash(op) => validate_bash_operation(command_name, op),
        #[cfg(feature = "sql")]
        OperationTemplate::Sql(op) => validate_sql_operation(command_name, op, allowed_secrets),
        _ => bail!("unsupported protocol (feature not enabled)"),
    }
}

#[cfg(feature = "http")]
fn validate_http_operation(
    command_name: &str,
    operation: &HttpOperationTemplate,
    allowed_secrets: &[String],
) -> Result<()> {
    if operation.url.trim().is_empty() {
        bail!("command {command_name} has empty operation.url");
    }
    if operation.method.trim().is_empty() {
        bail!("command {command_name} has empty operation.method");
    }

    if let Some(auth) = &operation.auth {
        validate_auth(command_name, auth, allowed_secrets)?;
    }
    if let Some(body) = &operation.body {
        validate_body(command_name, body)?;
    }
    validate_transport(command_name, operation.transport.as_ref())?;

    Ok(())
}

#[cfg(feature = "graphql")]
fn validate_graphql_operation(
    command_name: &str,
    operation: &GraphqlOperationTemplate,
    allowed_secrets: &[String],
) -> Result<()> {
    if operation.url.trim().is_empty() {
        bail!("command {command_name} has empty operation.url");
    }
    if operation.graphql.query.trim().is_empty() {
        bail!("command {command_name} has empty operation.graphql.query");
    }
    if !operation.method.trim().is_empty() && !operation.method.eq_ignore_ascii_case("POST") {
        bail!("command {command_name} graphql operation.method must be POST when provided");
    }

    if let Some(auth) = &operation.auth {
        validate_auth(command_name, auth, allowed_secrets)?;
    }
    validate_transport(command_name, operation.transport.as_ref())?;

    Ok(())
}

#[cfg(feature = "grpc")]
fn validate_grpc_operation(
    command_name: &str,
    operation: &GrpcOperationTemplate,
    allowed_secrets: &[String],
) -> Result<()> {
    if operation.url.trim().is_empty() {
        bail!("command {command_name} has empty operation.url");
    }
    if operation.grpc.service.trim().is_empty() {
        bail!("command {command_name} has empty operation.grpc.service");
    }
    if operation.grpc.method.trim().is_empty() {
        bail!("command {command_name} has empty operation.grpc.method");
    }
    if let Some(path) = &operation.grpc.descriptor_set_file
        && path.trim().is_empty()
    {
        bail!("command {command_name} operation.grpc.descriptor_set_file must not be empty");
    }
    if let Some(body) = &operation.grpc.body
        && !body.is_object()
        && !body.is_array()
    {
        bail!(
            "command {command_name} operation.grpc.body must be a JSON object (unary/server-streaming) or array (client-streaming)"
        );
    }

    if let Some(auth) = &operation.auth {
        if let AuthTemplate::ApiKey { location, .. } = auth
            && !matches!(location, ApiKeyLocation::Header)
        {
            bail!("command {command_name} grpc auth api_key location must be `header`");
        }
        validate_auth(command_name, auth, allowed_secrets)?;
    }
    if let Some(transport) = operation.transport.as_ref() {
        if transport.proxy_profile.is_some() {
            bail!("command {command_name} grpc transport.proxy_profile is not supported");
        }
        if transport
            .tls
            .as_ref()
            .and_then(|tls| tls.min_version.as_ref())
            .is_some()
        {
            bail!("command {command_name} grpc transport.tls.min_version is not supported");
        }
    }
    validate_transport(command_name, operation.transport.as_ref())?;

    Ok(())
}

fn validate_auth(
    command_name: &str,
    auth: &AuthTemplate,
    allowed_secrets: &[String],
) -> Result<()> {
    let ensure_secret = |secret: &String| -> Result<()> {
        if !allowed_secrets.iter().any(|s| s == secret) {
            bail!(
                "command {command_name} auth secret `{secret}` is not declared in annotations.secrets"
            );
        }
        Ok(())
    };

    match auth {
        AuthTemplate::None => {}
        AuthTemplate::ApiKey { secret, .. } => ensure_secret(secret)?,
        AuthTemplate::Bearer { secret } => ensure_secret(secret)?,
        AuthTemplate::Basic {
            password_secret, ..
        } => ensure_secret(password_secret)?,
        AuthTemplate::OAuth2Profile { .. } => {}
    }

    Ok(())
}

fn validate_body(command_name: &str, body: &BodyTemplate) -> Result<()> {
    match body {
        BodyTemplate::Multipart { parts } => {
            if parts.is_empty() {
                bail!("command {command_name} multipart body must include at least one part");
            }
            for part in parts {
                validate_part(command_name, part)?;
            }
        }
        BodyTemplate::FileStream { path, .. } => {
            if path.trim().is_empty() {
                bail!("command {command_name} file_stream body path must not be empty");
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_part(command_name: &str, part: &MultipartPartTemplate) -> Result<()> {
    let mut count = 0;
    if part.value.is_some() {
        count += 1;
    }
    if part.bytes_base64.is_some() {
        count += 1;
    }
    if part.file_path.is_some() {
        count += 1;
    }
    if count != 1 {
        bail!(
            "command {command_name} multipart part `{}` must specify exactly one of value, bytes_base64, file_path",
            part.name
        );
    }
    Ok(())
}

fn validate_transport(command_name: &str, transport: Option<&TransportTemplate>) -> Result<()> {
    let Some(transport) = transport else {
        return Ok(());
    };

    if let Some(timeout_ms) = transport.timeout_ms
        && timeout_ms == 0
    {
        bail!("command {command_name} transport.timeout_ms must be greater than 0");
    }

    if let Some(max_response_bytes) = transport.max_response_bytes
        && max_response_bytes == 0
    {
        bail!("command {command_name} transport.max_response_bytes must be greater than 0");
    }

    if let Some(proxy_profile) = transport.proxy_profile.as_ref()
        && proxy_profile.trim().is_empty()
    {
        bail!("command {command_name} transport.proxy_profile must not be empty");
    }

    if let Some(tls) = transport.tls.as_ref()
        && let Some(min_version) = tls.min_version.as_ref()
    {
        let min_version = min_version.trim();
        if !min_version.is_empty() && !matches!(min_version, "1.0" | "1.1" | "1.2" | "1.3") {
            bail!(
                "command {command_name} has unsupported transport.tls.min_version `{min_version}`"
            );
        }
    }

    Ok(())
}

// ── Bash validation ──────────────────────────────────────

#[cfg(feature = "bash")]
fn validate_bash_operation(command_name: &str, operation: &BashOperationTemplate) -> Result<()> {
    if operation.bash.script.trim().is_empty() {
        bail!("command {command_name} has empty operation.bash.script");
    }

    if let Some(sandbox) = &operation.bash.sandbox
        && let Some(writable_paths) = &sandbox.writable_paths
    {
        for path in writable_paths {
            if path.starts_with('/') || path.starts_with('\\') {
                bail!(
                    "command {command_name} operation.bash.sandbox.writable_paths contains absolute path `{path}`"
                );
            }
            if Path::new(path)
                .components()
                .any(|c| matches!(c, Component::ParentDir))
            {
                bail!(
                    "command {command_name} operation.bash.sandbox.writable_paths contains `..` in path `{path}`"
                );
            }
        }
    }

    validate_transport(command_name, operation.transport.as_ref())
}

// ── SQL validation ───────────────────────────────────────

#[cfg(feature = "sql")]
fn validate_sql_operation(
    command_name: &str,
    operation: &SqlOperationTemplate,
    allowed_secrets: &[String],
) -> Result<()> {
    if operation.sql.query.trim().is_empty() {
        bail!("command {command_name} has empty operation.sql.query");
    }

    if operation.sql.query.contains("{{") || operation.sql.query.contains("}}") {
        bail!(
            "command {command_name} operation.sql.query must not contain Jinja2 template expressions"
        );
    }

    if operation.sql.connection_secret.trim().is_empty() {
        bail!("command {command_name} has empty operation.sql.connection_secret");
    }

    if !allowed_secrets
        .iter()
        .any(|s| s == &operation.sql.connection_secret)
    {
        bail!(
            "command {command_name} operation.sql.connection_secret `{}` is not declared in annotations.secrets",
            operation.sql.connection_secret
        );
    }

    validate_transport(command_name, operation.transport.as_ref())
}
