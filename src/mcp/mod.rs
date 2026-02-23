pub mod auth;
pub mod policy;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequest, State},
    http::StatusCode,
    middleware as axum_middleware,
    response::IntoResponse,
    routing::{get, post},
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
};

use crate::auth::oauth2::OAuthManager;
use crate::config::{Config, PolicyRule};
use crate::expression::ast::CallExpression;
use crate::expression::binder::bind_arguments;
use crate::output::human::render_human_output;
use crate::output::json::render_json_output;
use crate::protocol::builder::build_prepared_request;
use crate::protocol::executor::execute_prepared_request;
use crate::secrets::SecretManager;
use crate::template::catalog::{TemplateCatalog, TemplateCatalogEntry};
use crate::template::schema::{CommandMode, ParamSpec, ParamType};

const JSONRPC_VERSION: &str = "2.0";
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";
const DISCOVERY_SEARCH_TOOL_NAME: &str = "earl.tool_search";
const DISCOVERY_CALL_TOOL_NAME: &str = "earl.tool_call";
const DEFAULT_DISCOVERY_LIMIT: usize = 10;
const MAX_DISCOVERY_LIMIT: usize = 50;

/// Maximum size of a single JSON-RPC frame over stdio or HTTP (10 MiB).
const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpMode {
    Full,
    Discovery,
}

#[derive(Debug, Clone)]
pub struct McpServerOptions {
    pub transport: ServerTransport,
    pub listen: SocketAddr,
    pub mode: McpMode,
    pub auto_yes: bool,
    pub allow_unauthenticated: bool,
}

#[derive(Clone)]
struct McpState {
    catalog: Arc<TemplateCatalog>,
    cfg: Config,
    mode: McpMode,
    auto_yes: bool,
    jwt: Option<auth::JwtState>,
    policies: Vec<PolicyRule>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    id: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION,
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug)]
struct RpcFailure {
    code: i64,
    message: String,
}

impl RpcFailure {
    fn access_denied(message: impl Into<String>) -> Self {
        Self {
            code: -32001,
            message: message.into(),
        }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }

    fn method_not_found(message: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: message.into(),
        }
    }

    fn internal(err: anyhow::Error) -> Self {
        Self {
            code: -32603,
            message: err.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    protocol_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct DiscoverySearchArgs {
    query: String,
    #[serde(default = "default_discovery_limit")]
    limit: usize,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DiscoveryInvokeArgs {
    name: String,
    #[serde(default)]
    arguments: Map<String, Value>,
}

pub async fn run_server(
    options: McpServerOptions,
    catalog: TemplateCatalog,
    cfg: Config,
) -> Result<()> {
    if options.transport == ServerTransport::Http {
        let has_jwt = cfg.auth.jwt.is_some();
        if has_jwt && options.allow_unauthenticated {
            bail!(
                "conflicting configuration: [auth.jwt] is configured but \
                 --allow-unauthenticated was also passed; remove one to proceed"
            );
        }
        if !has_jwt && !options.allow_unauthenticated {
            bail!(
                "HTTP transport requires authentication: configure [auth.jwt] in your \
                 config file, or pass --allow-unauthenticated to explicitly opt out \
                 (not recommended for production)"
            );
        }
    }

    let jwt = match &cfg.auth.jwt {
        Some(jwt_config) => Some(auth::JwtState::new(jwt_config).await?),
        None => None,
    };

    let policies = cfg.policy.clone();

    let state = McpState {
        catalog: Arc::new(catalog),
        cfg,
        mode: options.mode,
        auto_yes: options.auto_yes,
        jwt,
        policies,
    };

    match options.transport {
        ServerTransport::Stdio => run_stdio(state).await,
        ServerTransport::Http => run_http(state, options.listen).await,
    }
}

async fn run_stdio(state: McpState) -> Result<()> {
    let mut reader = BufReader::new(tokio::io::stdin());
    let mut writer = BufWriter::new(tokio::io::stdout());

    while let Some(frame) = read_stdio_frame(&mut reader).await? {
        let request = match serde_json::from_slice::<JsonRpcRequest>(&frame) {
            Ok(request) => request,
            Err(err) => {
                let response =
                    JsonRpcResponse::error(Value::Null, -32700, format!("parse error: {err}"));
                write_stdio_frame(&mut writer, &response).await?;
                continue;
            }
        };

        if let Some(response) = handle_request(request, &state, None).await {
            write_stdio_frame(&mut writer, &response).await?;
        }
    }

    writer.flush().await?;
    Ok(())
}

async fn run_http(state: McpState, listen: SocketAddr) -> Result<()> {
    let app = if let Some(jwt_state) = &state.jwt {
        let authenticated = Router::new()
            .route("/mcp", post(handle_http_rpc))
            .layer(axum_middleware::from_fn_with_state(
                jwt_state.clone(),
                auth::require_jwt_auth,
            ))
            .with_state(state);

        Router::new()
            .route("/health", get(|| async { StatusCode::NO_CONTENT }))
            .merge(authenticated)
            .layer(DefaultBodyLimit::max(MAX_FRAME_SIZE))
    } else {
        Router::new()
            .route("/mcp", post(handle_http_rpc))
            .route("/health", get(|| async { StatusCode::NO_CONTENT }))
            .with_state(state)
            .layer(DefaultBodyLimit::max(MAX_FRAME_SIZE))
    };

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed binding MCP HTTP listener on {listen}"))?;

    eprintln!("earl MCP server listening on http://{listen}/mcp");

    axum::serve(listener, app)
        .await
        .context("MCP HTTP server exited unexpectedly")
}

async fn handle_http_rpc(
    State(state): State<McpState>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    let subject = request.extensions().get::<auth::Subject>().cloned();
    if subject.is_none() && state.jwt.is_none() {
        tracing::warn!(
            target: "earl::audit",
            "processing unauthenticated HTTP request (--allow-unauthenticated is active)"
        );
    }
    let body: Json<JsonRpcRequest> = match axum::extract::Json::from_request(request, &state).await
    {
        Ok(json) => json,
        Err(err) => {
            let response =
                JsonRpcResponse::error(Value::Null, -32700, format!("parse error: {err}"));
            return (StatusCode::OK, Json(response)).into_response();
        }
    };

    match handle_request(body.0, &state, subject.as_ref()).await {
        Some(response) => (StatusCode::OK, Json(response)).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

async fn handle_request(
    request: JsonRpcRequest,
    state: &McpState,
    subject: Option<&auth::Subject>,
) -> Option<JsonRpcResponse> {
    let id = request.id.clone();

    if request.jsonrpc != JSONRPC_VERSION {
        return id.map(|id| JsonRpcResponse::error(id, -32600, "jsonrpc must be \"2.0\""));
    }

    let result = match request.method.as_str() {
        "initialize" => handle_initialize(request.params, state.mode),
        "notifications/initialized" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({
            "tools": build_tools(state, subject),
        })),
        "tools/call" => handle_tools_call(request.params, state, subject).await,
        _ => Err(RpcFailure::method_not_found(format!(
            "method `{}` not found",
            request.method
        ))),
    };

    id.map(|id| match result {
        Ok(value) => JsonRpcResponse::success(id, value),
        Err(err) => JsonRpcResponse::error(id, err.code, err.message),
    })
}

fn handle_initialize(
    params: Option<Value>,
    mode: McpMode,
) -> std::result::Result<Value, RpcFailure> {
    let params = decode_params::<InitializeParams>(params)?;
    let protocol_version = params
        .protocol_version
        .unwrap_or_else(|| DEFAULT_PROTOCOL_VERSION.to_string());

    let mut response = json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {
                "listChanged": false,
            }
        },
        "serverInfo": {
            "name": "earl",
            "version": env!("CARGO_PKG_VERSION"),
        }
    });

    if mode == McpMode::Discovery
        && let Some(object) = response.as_object_mut()
    {
        object.insert(
            "instructions".to_string(),
            Value::String(
                "Use `earl.tool_search` to discover relevant tools, then call `earl.tool_call` \
with the selected tool name and arguments."
                    .to_string(),
            ),
        );
    }

    Ok(response)
}

async fn handle_tools_call(
    params: Option<Value>,
    state: &McpState,
    subject: Option<&auth::Subject>,
) -> std::result::Result<Value, RpcFailure> {
    let params = decode_params::<ToolCallParams>(params)?;
    let tool_key = params.name.to_ascii_lowercase();

    match state.mode {
        McpMode::Full => {
            let entry = state.catalog.get(&tool_key).ok_or_else(|| {
                RpcFailure::invalid_params(format!("unknown tool `{}`", params.name))
            })?;

            // When unauthenticated, --yes is the only write gate
            if entry.mode == CommandMode::Write && !state.auto_yes && subject.is_none() {
                return Err(RpcFailure::access_denied(
                    "write-mode tools are disabled on this server instance",
                ));
            }

            if let Some(subject) = subject {
                let decision = policy::evaluate(&state.policies, &subject.0, &tool_key, entry.mode);
                let jti = subject.1.as_deref().unwrap_or("-");
                if decision == policy::PolicyDecision::Deny {
                    tracing::info!(
                        target: "earl::audit",
                        subject = %subject,
                        tool = %tool_key,
                        jti = jti,
                        decision = "deny",
                        "policy denied tool call"
                    );
                    return Err(RpcFailure::access_denied(format!(
                        "access denied: subject is not authorized to call `{}`",
                        tool_key
                    )));
                }
                tracing::info!(
                    target: "earl::audit",
                    subject = %subject,
                    tool = %tool_key,
                    jti = jti,
                    decision = "allow",
                    "policy allowed tool call"
                );
            }

            execute_template_tool(entry, params.arguments, state)
                .await
                .map_err(RpcFailure::internal)
        }
        McpMode::Discovery => {
            handle_discovery_tools_call(
                ToolCallParams {
                    name: params.name,
                    arguments: params.arguments,
                },
                state,
                subject,
            )
            .await
        }
    }
}

async fn handle_discovery_tools_call(
    params: ToolCallParams,
    state: &McpState,
    subject: Option<&auth::Subject>,
) -> std::result::Result<Value, RpcFailure> {
    match params.name.as_str() {
        DISCOVERY_SEARCH_TOOL_NAME => handle_discovery_search(params.arguments, state, subject),
        DISCOVERY_CALL_TOOL_NAME => handle_discovery_invoke(params.arguments, state, subject).await,
        _ => Err(RpcFailure::invalid_params(format!(
            "unknown discovery tool `{}`",
            params.name
        ))),
    }
}

fn handle_discovery_search(
    arguments: Map<String, Value>,
    state: &McpState,
    subject: Option<&auth::Subject>,
) -> std::result::Result<Value, RpcFailure> {
    let args = decode_argument_map::<DiscoverySearchArgs>(arguments)?;
    let query = args.query.trim();
    if query.is_empty() {
        return Err(RpcFailure::invalid_params("query must not be empty"));
    }

    let mode_filter = parse_mode_filter(args.mode.as_deref())?;
    let provider_filter = args
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let category_filter = args
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);

    let limit = args.limit.clamp(1, MAX_DISCOVERY_LIMIT);
    let mut ranked: Vec<(&TemplateCatalogEntry, f32)> = state
        .catalog
        .values()
        .filter(|entry| {
            mode_filter.is_none_or(|mode| entry.mode == mode)
                && provider_filter
                    .as_ref()
                    .is_none_or(|provider| entry.provider.to_ascii_lowercase() == *provider)
                && category_filter.as_ref().is_none_or(|category| {
                    entry
                        .categories
                        .iter()
                        .any(|candidate| candidate.to_ascii_lowercase() == *category)
                })
                // Access filter: hide tools the subject can't access or write
                // tools when --yes is off and unauthenticated
                && (if let Some(sub) = subject {
                    policy::evaluate(&state.policies, &sub.0, &entry.key, entry.mode)
                        == policy::PolicyDecision::Allow
                } else {
                    state.auto_yes || entry.mode != CommandMode::Write
                })
        })
        .map(|entry| (entry, discovery_score(query, entry)))
        .filter(|(_, score)| *score > 0.0)
        .collect();

    ranked.sort_by(|(left_entry, left_score), (right_entry, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_entry.key.cmp(&right_entry.key))
    });
    ranked.truncate(limit);

    let matches: Vec<Value> = ranked
        .iter()
        .map(|(entry, score)| {
            json!({
                "name": entry.key,
                "score": round_score(*score),
                "summary": entry.summary,
                "mode": entry.mode.as_str(),
                "categories": entry.categories,
                "tool": tool_from_entry(entry),
            })
        })
        .collect();

    let text = if matches.is_empty() {
        format!("No tools matched query `{query}`.")
    } else {
        let mut lines = Vec::new();
        for (index, tool) in matches.iter().enumerate() {
            let name = tool["name"].as_str().unwrap_or("unknown");
            let mode = tool["mode"].as_str().unwrap_or("unknown");
            let summary = tool["summary"].as_str().unwrap_or("");
            lines.push(format!("{:02}. {} ({mode}) - {}", index + 1, name, summary));
        }
        format!(
            "Found {} matching tools for `{query}`:\n{}",
            matches.len(),
            lines.join("\n")
        )
    };

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": text,
            }
        ],
        "structuredContent": {
            "query": query,
            "limit": limit,
            "matches": matches,
        },
    }))
}

async fn handle_discovery_invoke(
    arguments: Map<String, Value>,
    state: &McpState,
    subject: Option<&auth::Subject>,
) -> std::result::Result<Value, RpcFailure> {
    let args = decode_argument_map::<DiscoveryInvokeArgs>(arguments)?;
    let tool_key = args.name.to_ascii_lowercase();
    let entry = state.catalog.get(&tool_key).ok_or_else(|| {
        RpcFailure::invalid_params(format!("unknown template tool `{}`", args.name))
    })?;

    // When unauthenticated, --yes is the only write gate
    if entry.mode == CommandMode::Write && !state.auto_yes && subject.is_none() {
        return Err(RpcFailure::access_denied(
            "write-mode tools are disabled on this server instance",
        ));
    }

    if let Some(subject) = subject {
        let decision = policy::evaluate(&state.policies, &subject.0, &tool_key, entry.mode);
        let jti = subject.1.as_deref().unwrap_or("-");
        if decision == policy::PolicyDecision::Deny {
            tracing::info!(
                target: "earl::audit",
                subject = %subject,
                tool = %tool_key,
                jti = jti,
                decision = "deny",
                "policy denied discovery tool call"
            );
            return Err(RpcFailure::access_denied(format!(
                "access denied: subject is not authorized to call `{}`",
                tool_key
            )));
        }
        tracing::info!(
            target: "earl::audit",
            subject = %subject,
            tool = %tool_key,
            jti = jti,
            decision = "allow",
            "policy allowed discovery tool call"
        );
    }

    execute_template_tool(entry, args.arguments, state)
        .await
        .map_err(RpcFailure::internal)
}

async fn execute_template_tool(
    entry: &TemplateCatalogEntry,
    arguments: Map<String, Value>,
    state: &McpState,
) -> Result<Value> {
    let expression = CallExpression {
        provider: entry.provider.clone(),
        command: entry.command.clone(),
        positional_args: Vec::new(),
        named_args: arguments.into_iter().collect(),
    };

    let bound_args = bind_arguments(&expression, &entry.template.params)?;

    let secret_manager = SecretManager::new();
    let allow_rules = state.cfg.network.allow.clone();
    let proxy_profiles = state.cfg.network.proxy_profiles.clone();
    let sandbox_config = state.cfg.sandbox.clone();
    let oauth_manager = OAuthManager::new(state.cfg.clone(), SecretManager::new())?;

    let prepared = build_prepared_request(
        entry,
        bound_args,
        &secret_manager,
        &oauth_manager,
        &allow_rules,
        &proxy_profiles,
        &sandbox_config,
        None, // active_env — will be wired in Task 11
    )
    .await?;
    let execution = execute_prepared_request(&prepared)
        .await
        .map_err(|e| anyhow::anyhow!("{}", prepared.redactor.redact(&e.to_string())))?;

    let output = render_human_output(&prepared.result_template, &prepared.args, &execution.result)?;

    let structured = prepared
        .redactor
        .redact_json(&render_json_output(&execution));

    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": prepared.redactor.redact(&output),
            }
        ],
        "structuredContent": structured,
    }))
}

fn build_tools(state: &McpState, subject: Option<&auth::Subject>) -> Vec<Value> {
    match state.mode {
        McpMode::Full => state
            .catalog
            .values()
            .filter(|entry| {
                if let Some(sub) = subject {
                    // Authenticated: filter by policy (which handles mode restrictions)
                    policy::evaluate(&state.policies, &sub.0, &entry.key, entry.mode)
                        == policy::PolicyDecision::Allow
                } else {
                    // Unauthenticated: hide write tools when --yes is off
                    state.auto_yes || entry.mode != CommandMode::Write
                }
            })
            .map(tool_from_entry)
            .collect(),
        McpMode::Discovery => build_discovery_tools(),
    }
}

fn tool_from_entry(entry: &TemplateCatalogEntry) -> Value {
    let mode = match entry.mode {
        CommandMode::Read => "read",
        CommandMode::Write => "write",
    };

    let mut description = entry.summary.clone();
    if !entry.description.trim().is_empty() {
        description.push_str("\n\n");
        description.push_str(entry.description.trim());
    }
    description.push_str("\n\n");
    description.push_str(&format!("Mode: {mode}"));

    json!({
        "name": entry.key,
        "title": entry.title,
        "description": description,
        "inputSchema": input_schema_for(&entry.template.params),
    })
}

fn build_discovery_tools() -> Vec<Value> {
    vec![
        json!({
            "name": DISCOVERY_SEARCH_TOOL_NAME,
            "title": "Search Earl Tools",
            "description": "Find matching Earl template tools by natural-language query and optional filters.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural-language intent to match against tool metadata.",
                    },
                    "limit": {
                        "type": "integer",
                        "description": format!("Maximum matches to return (1-{}).", MAX_DISCOVERY_LIMIT),
                        "default": DEFAULT_DISCOVERY_LIMIT,
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["read", "write"],
                        "description": "Optional mode filter.",
                    },
                    "provider": {
                        "type": "string",
                        "description": "Optional provider filter (for example `github`).",
                    },
                    "category": {
                        "type": "string",
                        "description": "Optional category filter.",
                    },
                },
                "required": ["query"],
                "additionalProperties": false,
            },
        }),
        json!({
            "name": DISCOVERY_CALL_TOOL_NAME,
            "title": "Invoke Earl Tool",
            "description": "Execute a discovered Earl template tool by name.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Exact template tool name returned by earl.tool_search.",
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Arguments for the selected template tool.",
                        "default": {},
                    },
                },
                "required": ["name"],
                "additionalProperties": false,
            },
        }),
    ]
}

fn input_schema_for(params: &[ParamSpec]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for param in params {
        let mut schema = Map::new();
        schema.insert(
            "type".to_string(),
            Value::String(param_json_type(param.r#type).to_string()),
        );

        if let Some(description) = &param.description
            && !description.trim().is_empty()
        {
            schema.insert(
                "description".to_string(),
                Value::String(description.trim().to_string()),
            );
        }

        if let Some(default) = &param.default {
            schema.insert("default".to_string(), default.clone());
        }

        properties.insert(param.name.clone(), Value::Object(schema));

        if param.required {
            required.push(Value::String(param.name.clone()));
        }
    }

    let mut root = Map::new();
    root.insert("type".to_string(), Value::String("object".to_string()));
    root.insert("properties".to_string(), Value::Object(properties));
    root.insert("additionalProperties".to_string(), Value::Bool(false));
    if !required.is_empty() {
        root.insert("required".to_string(), Value::Array(required));
    }

    Value::Object(root)
}

fn param_json_type(kind: ParamType) -> &'static str {
    match kind {
        ParamType::String => "string",
        ParamType::Integer => "integer",
        ParamType::Number => "number",
        ParamType::Boolean => "boolean",
        ParamType::Null => "null",
        ParamType::Array => "array",
        ParamType::Object => "object",
    }
}

fn parse_mode_filter(mode: Option<&str>) -> std::result::Result<Option<CommandMode>, RpcFailure> {
    match mode.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some("read") => Ok(Some(CommandMode::Read)),
        Some("write") => Ok(Some(CommandMode::Write)),
        Some(other) => Err(RpcFailure::invalid_params(format!(
            "invalid mode filter `{other}`; expected `read` or `write`"
        ))),
    }
}

fn discovery_score(query: &str, entry: &TemplateCatalogEntry) -> f32 {
    let query = query.to_ascii_lowercase();
    if query.is_empty() {
        return 0.0;
    }

    let key = entry.key.to_ascii_lowercase();
    let provider = entry.provider.to_ascii_lowercase();
    let command = entry.command.to_ascii_lowercase();
    let title = entry.title.to_ascii_lowercase();
    let summary = entry.summary.to_ascii_lowercase();
    let description = entry.description.to_ascii_lowercase();
    let categories: Vec<String> = entry
        .categories
        .iter()
        .map(|category| category.to_ascii_lowercase())
        .collect();
    let params: Vec<String> = entry
        .template
        .params
        .iter()
        .map(|param| param.name.to_ascii_lowercase())
        .collect();

    let mut score = 0.0;
    if key == query {
        score += 12.0;
    }
    if key.contains(&query) {
        score += 6.0;
    }
    if title.contains(&query) {
        score += 4.0;
    }
    if summary.contains(&query) {
        score += 3.0;
    }
    if description.contains(&query) {
        score += 2.0;
    }

    for token in query.split_whitespace() {
        if key.contains(token) {
            score += 2.0;
        }
        if provider == token {
            score += 2.0;
        }
        if command == token {
            score += 2.0;
        }
        if title.contains(token) {
            score += 1.5;
        }
        if summary.contains(token) {
            score += 1.0;
        }
        if description.contains(token) {
            score += 0.5;
        }
        if categories.iter().any(|category| category.contains(token)) {
            score += 1.0;
        }
        if params.iter().any(|param| param == token) {
            score += 0.5;
        }
    }

    score
}

fn round_score(score: f32) -> f32 {
    (score * 1000.0).round() / 1000.0
}

fn default_discovery_limit() -> usize {
    DEFAULT_DISCOVERY_LIMIT
}

fn decode_params<T: DeserializeOwned>(params: Option<Value>) -> std::result::Result<T, RpcFailure> {
    match params {
        Some(value) => serde_json::from_value(value)
            .map_err(|err| RpcFailure::invalid_params(format!("invalid params: {err}"))),
        None => serde_json::from_value(Value::Object(Map::new()))
            .map_err(|err| RpcFailure::invalid_params(format!("invalid params: {err}"))),
    }
}

fn decode_argument_map<T: DeserializeOwned>(
    arguments: Map<String, Value>,
) -> std::result::Result<T, RpcFailure> {
    serde_json::from_value(Value::Object(arguments))
        .map_err(|err| RpcFailure::invalid_params(format!("invalid params: {err}")))
}

async fn read_stdio_frame<R: AsyncBufRead + Unpin>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            return Ok(None);
        }

        if line.trim().is_empty() {
            continue;
        }

        if let Some(content_length) = parse_content_length(&line)? {
            if content_length > MAX_FRAME_SIZE {
                bail!(
                    "Content-Length {content_length} exceeds maximum frame size ({MAX_FRAME_SIZE} bytes)"
                );
            }

            loop {
                let mut header = String::new();
                let read = reader.read_line(&mut header).await?;
                if read == 0 {
                    bail!("unexpected EOF while reading stdio headers");
                }
                if header == "\r\n" || header == "\n" {
                    break;
                }
            }

            let mut body = vec![0_u8; content_length];
            reader.read_exact(&mut body).await?;
            return Ok(Some(body));
        }

        let payload = line.trim_end_matches(['\r', '\n']).as_bytes().to_vec();
        return Ok(Some(payload));
    }
}

fn parse_content_length(line: &str) -> Result<Option<usize>> {
    let (name, value) = match line.split_once(':') {
        Some(parts) => parts,
        None => return Ok(None),
    };

    if !name.trim().eq_ignore_ascii_case("Content-Length") {
        return Ok(None);
    }

    let len = value
        .trim()
        .parse::<usize>()
        .with_context(|| format!("invalid Content-Length header `{line}`"))?;

    Ok(Some(len))
}

async fn write_stdio_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &JsonRpcResponse,
) -> Result<()> {
    let body = serde_json::to_vec(response)?;
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
        .await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(all(test, feature = "http"))]
mod tests {
    use std::collections::BTreeMap;
    use std::io::Cursor;
    use std::path::PathBuf;

    use super::*;
    use crate::config::PolicyEffect;
    use crate::template::catalog::{TemplateScope, TemplateSource};
    use crate::template::schema::{
        Annotations, CommandTemplate, HttpOperationTemplate, OperationTemplate, ResultDecode,
        ResultTemplate,
    };

    fn test_state(entries: Vec<TemplateCatalogEntry>, auto_yes: bool) -> McpState {
        test_state_with_mode(entries, auto_yes, McpMode::Full)
    }

    fn test_state_with_mode(
        entries: Vec<TemplateCatalogEntry>,
        auto_yes: bool,
        mode: McpMode,
    ) -> McpState {
        let mut catalog = TemplateCatalog::empty();
        for entry in entries {
            catalog.upsert(entry.key.clone(), entry);
        }

        McpState {
            catalog: Arc::new(catalog),
            cfg: Config::default(),
            mode,
            auto_yes,
            jwt: None,
            policies: Vec::new(),
        }
    }

    fn sample_entry(key: &str, mode: CommandMode, params: Vec<ParamSpec>) -> TemplateCatalogEntry {
        let (provider, command) = key.split_once('.').expect("tool key format");

        TemplateCatalogEntry {
            key: key.to_string(),
            provider: provider.to_string(),
            command: command.to_string(),
            title: "Demo title".to_string(),
            summary: "Demo summary".to_string(),
            description: "Demo description".to_string(),
            categories: vec!["demo".to_string()],
            mode,
            source: TemplateSource {
                path: PathBuf::from("templates/demo.hcl"),
                scope: TemplateScope::Local,
            },
            template: CommandTemplate {
                title: "Demo title".to_string(),
                summary: "Demo summary".to_string(),
                description: "Demo description".to_string(),
                categories: vec![],
                annotations: Annotations {
                    mode,
                    secrets: vec![],
                    allow_environment_protocol_switching: false,
                },
                params,
                operation: OperationTemplate::Http(HttpOperationTemplate {
                    method: "GET".to_string(),
                    url: "https://api.example.com".to_string(),
                    path: None,
                    query: None,
                    headers: None,
                    cookies: None,
                    auth: None,
                    body: None,
                    stream: false,
                    transport: None,
                }),
                result: ResultTemplate {
                    decode: ResultDecode::Auto,
                    extract: None,
                    output: "{{ result }}".to_string(),
                    result_alias: None,
                },
                environment_overrides: BTreeMap::new(),
            },
            provider_environments: None,
        }
    }

    #[tokio::test]
    async fn initialize_returns_tools_capability() {
        let state = test_state(Vec::new(), false);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
            })),
            id: Some(json!(1)),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let result = response.result.expect("result");

        assert_eq!(result["serverInfo"]["name"], "earl");
        assert_eq!(result["capabilities"]["tools"]["listChanged"], false);
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn initialize_includes_discovery_instructions() {
        let state = test_state_with_mode(Vec::new(), false, McpMode::Discovery);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: Some(json!({})),
            id: Some(json!(1)),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let result = response.result.expect("result");

        assert!(
            result["instructions"]
                .as_str()
                .expect("instructions")
                .contains(DISCOVERY_SEARCH_TOOL_NAME)
        );
    }

    #[tokio::test]
    async fn tools_list_exposes_catalog_entries() {
        let params = vec![ParamSpec {
            name: "repo".to_string(),
            r#type: ParamType::String,
            required: true,
            default: None,
            description: Some("Repository name".to_string()),
        }];

        let state = test_state(
            vec![sample_entry(
                "github.search_issues",
                CommandMode::Read,
                params,
            )],
            false,
        );

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: Some(json!({})),
            id: Some(json!("list-1")),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let result = response.result.expect("result");
        let tools = result["tools"].as_array().expect("tools array");

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "github.search_issues");
        assert_eq!(tools[0]["inputSchema"]["required"][0], "repo");
        assert_eq!(
            tools[0]["inputSchema"]["properties"]["repo"]["type"],
            "string"
        );
    }

    #[tokio::test]
    async fn discovery_mode_tools_list_exposes_discovery_tools() {
        let state = test_state_with_mode(
            vec![sample_entry(
                "github.search_issues",
                CommandMode::Read,
                Vec::new(),
            )],
            false,
            McpMode::Discovery,
        );

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: Some(json!({})),
            id: Some(json!("list-2")),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let result = response.result.expect("result");
        let tools = result["tools"].as_array().expect("tools array");
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect();

        assert_eq!(tools.len(), 2);
        assert!(names.contains(&DISCOVERY_SEARCH_TOOL_NAME));
        assert!(names.contains(&DISCOVERY_CALL_TOOL_NAME));
        assert!(!names.contains(&"github.search_issues"));
    }

    #[tokio::test]
    async fn discovery_tool_search_returns_ranked_matches() {
        let state = test_state_with_mode(
            vec![
                sample_entry("github.search_issues", CommandMode::Read, Vec::new()),
                sample_entry("github.create_issue", CommandMode::Write, Vec::new()),
            ],
            false,
            McpMode::Discovery,
        );

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": DISCOVERY_SEARCH_TOOL_NAME,
                "arguments": {
                    "query": "search issues",
                    "limit": 1
                },
            })),
            id: Some(json!("search-1")),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let result = response.result.expect("result");
        let matches = result["structuredContent"]["matches"]
            .as_array()
            .expect("matches");

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["name"], "github.search_issues");
        assert!(
            result["content"][0]["text"]
                .as_str()
                .expect("content text")
                .contains("Found 1 matching tools")
        );
    }

    #[tokio::test]
    async fn tools_call_rejects_unknown_tool() {
        let state = test_state(Vec::new(), false);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "missing.tool",
                "arguments": {},
            })),
            id: Some(json!(2)),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let error = response.error.expect("error");

        assert_eq!(error.code, -32602);
        assert!(error.message.contains("unknown tool"));
    }

    #[tokio::test]
    async fn discovery_tool_call_rejects_unknown_template_tool() {
        let state = test_state_with_mode(Vec::new(), false, McpMode::Discovery);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": DISCOVERY_CALL_TOOL_NAME,
                "arguments": {
                    "name": "missing.tool",
                    "arguments": {},
                },
            })),
            id: Some(json!("call-1")),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let error = response.error.expect("error");

        assert_eq!(error.code, -32602);
        assert!(error.message.contains("unknown template tool"));
    }

    #[tokio::test]
    async fn tools_call_blocks_write_tools_without_yes_flag() {
        let state = test_state(
            vec![sample_entry(
                "github.create_issue",
                CommandMode::Write,
                Vec::new(),
            )],
            false,
        );

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "github.create_issue",
                "arguments": {},
            })),
            id: Some(json!(3)),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let error = response.error.expect("error");

        assert_eq!(error.code, -32001);
        assert!(error.message.contains("write-mode tools are disabled"));
    }

    #[tokio::test]
    async fn read_stdio_frame_supports_content_length_messages() {
        let payload = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let framed = format!(
            "Content-Length: {}\r\n\r\n{}",
            payload.len(),
            String::from_utf8(payload.to_vec()).unwrap()
        );

        let mut reader = BufReader::new(Cursor::new(framed.into_bytes()));
        let frame = read_stdio_frame(&mut reader)
            .await
            .expect("frame read")
            .expect("frame");

        assert_eq!(frame, payload);
    }

    #[tokio::test]
    async fn read_stdio_frame_supports_newline_delimited_messages() {
        let payload = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let mut reader = BufReader::new(Cursor::new(format!(
            "{}\n",
            String::from_utf8(payload.to_vec()).unwrap()
        )));

        let frame = read_stdio_frame(&mut reader)
            .await
            .expect("frame read")
            .expect("frame");

        assert_eq!(frame, payload);
    }

    #[tokio::test]
    async fn tools_list_filters_by_policy() {
        let mut state = test_state(
            vec![
                sample_entry("github.search_issues", CommandMode::Read, Vec::new()),
                sample_entry("github.create_issue", CommandMode::Write, Vec::new()),
                sample_entry("slack.send_message", CommandMode::Write, Vec::new()),
            ],
            true,
        );
        state.policies = vec![PolicyRule {
            subjects: vec!["alice".to_string()],
            tools: vec!["github.*".to_string()],
            modes: None,
            effect: PolicyEffect::Allow,
        }];

        let subject = auth::Subject("alice".to_string(), None);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: Some(json!({})),
            id: Some(json!("list-policy")),
        };

        let response = handle_request(request, &state, Some(&subject))
            .await
            .expect("response");
        let result = response.result.expect("result");
        let tools = result["tools"].as_array().expect("tools array");
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"github.search_issues"));
        assert!(names.contains(&"github.create_issue"));
        assert!(!names.contains(&"slack.send_message"));
    }

    #[tokio::test]
    async fn tools_call_denied_by_policy() {
        let mut state = test_state(
            vec![sample_entry(
                "github.create_issue",
                CommandMode::Write,
                Vec::new(),
            )],
            true,
        );
        state.policies = vec![PolicyRule {
            subjects: vec!["bob".to_string()],
            tools: vec!["slack.*".to_string()],
            modes: None,
            effect: PolicyEffect::Allow,
        }];

        let subject = auth::Subject("bob".to_string(), None);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "github.create_issue",
                "arguments": {},
            })),
            id: Some(json!("call-policy")),
        };

        let response = handle_request(request, &state, Some(&subject))
            .await
            .expect("response");
        let error = response.error.expect("error");
        assert_eq!(error.code, -32001);
        assert!(error.message.contains("access denied"));
    }

    #[tokio::test]
    async fn no_subject_means_no_policy_filtering() {
        let mut state = test_state(
            vec![
                sample_entry("github.search_issues", CommandMode::Read, Vec::new()),
                sample_entry("slack.send_message", CommandMode::Write, Vec::new()),
            ],
            true,
        );
        state.policies = vec![PolicyRule {
            subjects: vec!["alice".to_string()],
            tools: vec!["github.*".to_string()],
            modes: None,
            effect: PolicyEffect::Allow,
        }];

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: Some(json!({})),
            id: Some(json!("list-no-subject")),
        };

        let response = handle_request(request, &state, None)
            .await
            .expect("response");
        let result = response.result.expect("result");
        let tools = result["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 2);
    }

    #[tokio::test]
    async fn discovery_search_filters_by_policy() {
        let mut state = test_state_with_mode(
            vec![
                sample_entry("github.search_issues", CommandMode::Read, Vec::new()),
                sample_entry("github.create_issue", CommandMode::Write, Vec::new()),
                sample_entry("slack.send_message", CommandMode::Write, Vec::new()),
            ],
            true,
            McpMode::Discovery,
        );
        state.policies = vec![PolicyRule {
            subjects: vec!["alice".to_string()],
            tools: vec!["github.*".to_string()],
            modes: None,
            effect: PolicyEffect::Allow,
        }];

        let subject = auth::Subject("alice".to_string(), None);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": DISCOVERY_SEARCH_TOOL_NAME,
                "arguments": {
                    "query": "send message",
                    "limit": 10
                },
            })),
            id: Some(json!("search-policy")),
        };

        let response = handle_request(request, &state, Some(&subject))
            .await
            .expect("response");
        let result = response.result.expect("result");
        let matches = result["structuredContent"]["matches"]
            .as_array()
            .expect("matches");
        let names: Vec<&str> = matches
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect();
        // slack.send_message should be filtered out by policy
        assert!(!names.contains(&"slack.send_message"));
    }
}
