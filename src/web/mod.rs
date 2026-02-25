use std::path::PathBuf;

use anyhow::Result;
use axum::extract::{Path as AxumPath, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::oauth2::OAuthManager;
use crate::config::{self, Config};
use crate::expression::ast::CallExpression;
use crate::expression::binder::bind_arguments;
use crate::output::human::render_human_output;
use crate::protocol::builder::build_prepared_request;
use crate::protocol::executor::execute_prepared_request;
use crate::secrets::SecretManager;
use crate::template::catalog::{TemplateCatalogEntry, TemplateScope};
use crate::template::loader::load_catalog;
use crate::template::schema::{CommandMode, OperationProtocol, ParamSpec};

static WEB_DIST: Dir<'static> = include_dir!("$OUT_DIR/web-build/web/dist");

#[derive(Debug, Clone)]
pub struct WebState {
    pub cwd: PathBuf,
    pub bearer_token: String,
}

pub fn build_router(state: WebState) -> Router {
    let authenticated_api = Router::new()
        .route("/api/tools", get(api_list_tools))
        .route("/api/tools/{key}", get(api_get_tool))
        .route("/api/execute", post(api_execute))
        .route("/api/validate", post(api_validate))
        .route("/api/secrets/status", post(api_secrets_status))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer_auth,
        ));

    Router::new()
        .route("/api/health", get(api_health))
        .merge(authenticated_api)
        .route("/", get(serve_index))
        .route("/{*path}", get(serve_static_or_index))
        .with_state(state)
}

async fn require_bearer_auth(
    State(state): State<WebState>,
    request: Request,
    next: middleware::Next,
) -> Response {
    let expected = format!("Bearer {}", state.bearer_token);
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == expected)
        .unwrap_or(false);

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": {
                    "code": "unauthorized",
                    "message": "missing or invalid Bearer token"
                }
            })),
        )
            .into_response()
    }
}

pub fn generate_bearer_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    use base64::Engine;

    let mut bytes = [0u8; 32];
    for chunk in bytes.chunks_mut(8) {
        let hash = RandomState::new().build_hasher().finish();
        chunk.copy_from_slice(&hash.to_ne_bytes());
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

async fn api_health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn api_list_tools(
    State(state): State<WebState>,
) -> Result<Json<Vec<ToolSummaryResponse>>, ApiError> {
    let catalog = load_catalog(&state.cwd).map_err(|err| {
        ApiError::execution_error(format!("failed loading template catalog: {err:#}"))
    })?;

    let mut tools: Vec<_> = catalog.values().map(tool_summary_from_entry).collect();
    tools.sort_by(|left, right| left.key.cmp(&right.key));

    Ok(Json(tools))
}

async fn api_get_tool(
    State(state): State<WebState>,
    AxumPath(key): AxumPath<String>,
) -> Result<Json<ToolDetailResponse>, ApiError> {
    let catalog = load_catalog(&state.cwd).map_err(|err| {
        ApiError::execution_error(format!("failed loading template catalog: {err:#}"))
    })?;

    let entry = catalog
        .get(&key)
        .ok_or_else(|| ApiError::unknown_command(format!("unknown command `{key}`")))?;

    let summary = tool_summary_from_entry(entry);
    let operation = serde_json::to_value(&entry.template.operation)
        .map_err(|err| ApiError::execution_error(format!("failed serializing operation: {err}")))?;

    Ok(Json(ToolDetailResponse { summary, operation }))
}

async fn api_execute(
    State(state): State<WebState>,
    Json(payload): Json<ExecuteRequest>,
) -> Result<Json<ExecuteSuccessResponse>, ApiError> {
    let catalog = load_catalog(&state.cwd).map_err(|err| {
        ApiError::execution_error(format!("failed loading template catalog: {err:#}"))
    })?;

    let entry = catalog.get(&payload.command).ok_or_else(|| {
        ApiError::unknown_command(format!("unknown command `{}`", payload.command))
    })?;

    let expression = CallExpression {
        provider: entry.provider.clone(),
        command: entry.command.clone(),
        positional_args: vec![],
        named_args: payload.args.into_iter().collect(),
    };

    let args = bind_arguments(&expression, &entry.template.params)
        .map_err(|err| ApiError::bind_error(err.to_string()))?;

    if entry.mode == CommandMode::Write && !payload.confirm_write {
        return Err(ApiError::write_confirmation_required(format!(
            "command `{}` is write-enabled and requires confirm_write=true",
            entry.key
        )));
    }

    let cfg = config::load_config()
        .map_err(|err| ApiError::execution_error(format!("failed loading config: {err:#}")))?;

    let success = execute_entry(entry, args, cfg)
        .await
        .map_err(|err| ApiError::execution_error(err.to_string()))?;

    Ok(Json(success))
}

async fn execute_entry(
    entry: &TemplateCatalogEntry,
    args: serde_json::Map<String, Value>,
    cfg: Config,
) -> Result<ExecuteSuccessResponse> {
    let allow_rules = cfg.network.allow.clone();
    let proxy_profiles = cfg.network.proxy_profiles.clone();
    let sandbox_config = cfg.sandbox.clone();

    let secret_manager = SecretManager::new();
    let oauth_manager = OAuthManager::new(cfg, SecretManager::new())?; // OAuthManager takes ownership, so a separate instance is needed

    let prepared = build_prepared_request(
        entry,
        args,
        &secret_manager,
        &oauth_manager,
        &allow_rules,
        &proxy_profiles,
        &sandbox_config,
        None, // active_env — web mode doesn't support environments
    )
    .await?;

    let execution = execute_prepared_request(&prepared).await?;
    let human_output =
        render_human_output(&prepared.result_template, &prepared.args, &execution.result)?;

    Ok(ExecuteSuccessResponse {
        key: entry.key.clone(),
        mode: entry.mode.as_str().to_string(),
        status: execution.status,
        url: prepared.redactor.redact(&execution.url),
        result: prepared.redactor.redact_json(&execution.result),
        decoded: prepared.redactor.redact_json(&execution.decoded),
        human_output: prepared.redactor.redact(&human_output),
    })
}

#[derive(Debug, Deserialize)]
struct ValidateRequest {
    command: String,
    #[serde(default)]
    args: serde_json::Map<String, Value>,
}

#[derive(Debug, Serialize)]
struct ValidateResponse {
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bound_params: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ValidateError>,
}

#[derive(Debug, Serialize)]
struct ValidateError {
    code: String,
    message: String,
}

async fn api_validate(
    State(state): State<WebState>,
    Json(payload): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, ApiError> {
    let catalog = load_catalog(&state.cwd).map_err(|err| {
        ApiError::execution_error(format!("failed loading template catalog: {err:#}"))
    })?;

    let key = &payload.command;
    let entry = match catalog.get(key.as_str()) {
        Some(entry) => entry,
        None => {
            let message = format!("unknown command `{key}`");
            return Ok(Json(ValidateResponse {
                valid: false,
                command_key: Some(key.clone()),
                bound_params: None,
                missing_required: None,
                error: Some(ValidateError {
                    code: "unknown_command".to_string(),
                    message,
                }),
            }));
        }
    };

    let expression = CallExpression {
        provider: entry.provider.clone(),
        command: entry.command.clone(),
        positional_args: vec![],
        named_args: payload.args.into_iter().collect(),
    };

    match bind_arguments(&expression, &entry.template.params) {
        Ok(args) => {
            let bound: Vec<String> = args.keys().cloned().collect();
            Ok(Json(ValidateResponse {
                valid: true,
                command_key: Some(key.clone()),
                bound_params: Some(bound),
                missing_required: Some(vec![]),
                error: None,
            }))
        }
        Err(err) => Ok(Json(ValidateResponse {
            valid: false,
            command_key: Some(key.clone()),
            bound_params: None,
            missing_required: None,
            error: Some(ValidateError {
                code: "bind_error".to_string(),
                message: err.to_string(),
            }),
        })),
    }
}

#[derive(Debug, Deserialize)]
struct SecretsStatusRequest {
    secrets: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SecretStatusResponse {
    configured: bool,
}

async fn api_secrets_status(
    Json(payload): Json<SecretsStatusRequest>,
) -> Json<std::collections::HashMap<String, SecretStatusResponse>> {
    let manager = SecretManager::new();
    let mut result = std::collections::HashMap::new();
    for secret_name in payload.secrets {
        let configured = manager.get(&secret_name).ok().flatten().is_some();
        result.insert(secret_name, SecretStatusResponse { configured });
    }
    Json(result)
}

async fn serve_index(State(state): State<WebState>) -> Response {
    serve_embedded_file("index.html", true, Some(&state.bearer_token))
}

async fn serve_static_or_index(
    State(state): State<WebState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        return serve_embedded_file("index.html", true, Some(&state.bearer_token));
    }

    if WEB_DIST.get_file(path).is_some() {
        return serve_embedded_file(path, false, None);
    }

    serve_embedded_file("index.html", true, Some(&state.bearer_token))
}

fn serve_embedded_file(path: &str, is_index: bool, token: Option<&str>) -> Response {
    let Some(file) = WEB_DIST.get_file(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mut headers = HeaderMap::new();
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type.as_ref())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );

    if is_index {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    } else if path.starts_with("assets/") {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }

    // Inject token meta tag for index.html
    if is_index && let (Some(token), Ok(html)) = (token, std::str::from_utf8(file.contents())) {
        let injected = html.replace(
            "</head>",
            &format!(r#"<meta name="earl-token" content="{token}"></head>"#),
        );
        return (headers, injected.into_bytes()).into_response();
    }

    (headers, file.contents().to_vec()).into_response()
}

fn tool_summary_from_entry(entry: &TemplateCatalogEntry) -> ToolSummaryResponse {
    ToolSummaryResponse {
        key: entry.key.clone(),
        provider: entry.provider.clone(),
        command: entry.command.clone(),
        title: entry.title.clone(),
        summary: entry.summary.clone(),
        description: entry.description.clone(),
        mode: entry.mode.as_str().to_string(),
        protocol: protocol_label(entry.template.operation.protocol()).to_string(),
        categories: entry.categories.clone(),
        secrets: entry.template.annotations.secrets.clone(),
        params: entry.template.params.clone(),
        source: ToolSourceResponse {
            scope: template_scope_label(entry.source.scope).to_string(),
            path: entry.source.path.display().to_string(),
        },
        example_cli: build_example_cli(entry),
    }
}

fn build_example_cli(entry: &TemplateCatalogEntry) -> String {
    if entry.template.params.is_empty() {
        return format!("earl call {}", entry.key);
    }

    let args = entry
        .template
        .params
        .iter()
        .map(example_cli_arg)
        .collect::<Vec<_>>()
        .join(" ");

    format!("earl call {} {args}", entry.key)
}

fn example_cli_arg(param: &ParamSpec) -> String {
    let value = param
        .default
        .as_ref()
        .map(cli_example_value)
        .unwrap_or_else(|| cli_placeholder_value(param.r#type));
    format!("--{} {value}", param.name)
}

fn cli_example_value(value: &Value) -> String {
    match value {
        Value::String(text) => shell_quote(text),
        Value::Array(_) | Value::Object(_) => {
            shell_quote(&serde_json::to_string(value).unwrap_or_default())
        }
        _ => value.to_string(),
    }
}

fn cli_placeholder_value(param_type: crate::template::schema::ParamType) -> String {
    match param_type {
        crate::template::schema::ParamType::String => "\"\"".to_string(),
        crate::template::schema::ParamType::Integer => "0".to_string(),
        crate::template::schema::ParamType::Number => "0".to_string(),
        crate::template::schema::ParamType::Boolean => "true".to_string(),
        crate::template::schema::ParamType::Null => "null".to_string(),
        crate::template::schema::ParamType::Array => "'[]'".to_string(),
        crate::template::schema::ParamType::Object => "'{}'".to_string(),
    }
}

fn shell_quote(s: &str) -> String {
    if s.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else if s.is_empty() {
        "\"\"".to_string()
    } else {
        s.to_string()
    }
}

fn template_scope_label(scope: TemplateScope) -> &'static str {
    match scope {
        TemplateScope::Local => "local",
        TemplateScope::Global => "global",
    }
}

#[allow(unreachable_patterns)]
fn protocol_label(protocol: OperationProtocol) -> &'static str {
    match protocol {
        #[cfg(feature = "http")]
        OperationProtocol::Http => "http",
        #[cfg(feature = "graphql")]
        OperationProtocol::Graphql => "graphql",
        #[cfg(feature = "grpc")]
        OperationProtocol::Grpc => "grpc",
        #[cfg(feature = "bash")]
        OperationProtocol::Bash => "bash",
        #[cfg(feature = "sql")]
        OperationProtocol::Sql => "sql",
        _ => "unknown",
    }
}

#[derive(Debug, Serialize)]
struct ToolSummaryResponse {
    key: String,
    provider: String,
    command: String,
    title: String,
    summary: String,
    description: String,
    mode: String,
    protocol: String,
    categories: Vec<String>,
    secrets: Vec<String>,
    params: Vec<ParamSpec>,
    source: ToolSourceResponse,
    example_cli: String,
}

#[derive(Debug, Serialize)]
struct ToolDetailResponse {
    #[serde(flatten)]
    summary: ToolSummaryResponse,
    operation: Value,
}

#[derive(Debug, Serialize)]
struct ToolSourceResponse {
    scope: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    command: String,
    #[serde(default)]
    args: serde_json::Map<String, Value>,
    #[serde(default)]
    confirm_write: bool,
}

#[derive(Debug, Serialize)]
struct ExecuteSuccessResponse {
    key: String,
    mode: String,
    status: u16,
    url: String,
    result: Value,
    decoded: Value,
    human_output: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn unknown_command(message: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "unknown_command",
            message,
        }
    }

    fn bind_error(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bind_error",
            message,
        }
    }

    fn write_confirmation_required(message: String) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "write_confirmation_required",
            message,
        }
    }

    fn execution_error(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "execution_error",
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": {
                    "code": self.code,
                    "message": self.message,
                }
            })),
        )
            .into_response()
    }
}

#[cfg(all(test, feature = "bash"))]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use serde_json::Value;
    use tower::util::ServiceExt;

    use super::{WebState, build_router};

    const TEST_TOKEN: &str = "test-token";

    const READ_TEMPLATE: &str = r#"
version = 1
provider = "demo"

command "echo" {
  title = "Echo"
  summary = "Echo text"
  description = "Prints the provided value."

  annotations {
    mode = "read"
    secrets = []
  }

  param "value" {
    type = "string"
    required = true
  }

  operation {
    protocol = "bash"
    bash {
      script = "printf '%s' '{{ args.value }}'"
    }
  }

  result {
    decode = "text"
    output = "{{ result }}"
  }
}
"#;

    const WRITE_TEMPLATE: &str = r#"
version = 1
provider = "demo"

command "write_echo" {
  title = "Write Echo"
  summary = "Write operation"
  description = "Write mode command."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "bash"
    bash {
      script = "printf '%s' 'ok'"
    }
  }

  result {
    decode = "text"
    output = "{{ result }}"
  }
}
"#;

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn registered_command_appears_in_tools_list() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/tools")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), 200);

            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();

            let tools = parsed.as_array().unwrap();
            assert_eq!(tools.len(), 1);
            assert_eq!(tools[0]["key"], "demo.echo");
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn registered_command_has_correct_protocol_label() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/tools")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), 200);

            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();
            let tools = parsed.as_array().unwrap();
            assert_eq!(tools[0]["protocol"], "bash");
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn example_cli_includes_command_key() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/tools")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();
            let tools = parsed.as_array().unwrap();
            let example = tools[0]["example_cli"].as_str().unwrap();
            assert!(example.contains("earl call demo.echo"));
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn example_cli_includes_required_param_flag() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .uri("/api/tools")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), 200);

            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();
            let tools = parsed.as_array().unwrap();
            let example = tools[0]["example_cli"].as_str().unwrap();
            assert!(example.contains("--value"));
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn execute_rejects_write_mode_without_confirmation() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", WRITE_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/execute")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::from(
                            serde_json::json!({
                                "command": "demo.write_echo"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), 403);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(parsed["error"]["code"], "write_confirmation_required");
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn execute_response_includes_command_key() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/execute")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::from(
                            serde_json::json!({
                                "command": "demo.echo",
                                "args": {"value": "hello"},
                                "confirm_write": false
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), 200);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();

            assert_eq!(parsed["key"], "demo.echo");
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn execute_response_mode_matches_template_annotation() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/execute")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::from(
                            serde_json::json!({
                                "command": "demo.echo",
                                "args": {"value": "hello"},
                                "confirm_write": false
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), 200);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(parsed["mode"], "read");
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn read_command_returns_human_output() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/execute")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::from(
                            serde_json::json!({
                                "command": "demo.echo",
                                "args": {"value": "hello"},
                                "confirm_write": false
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), 200);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();

            assert_eq!(parsed["human_output"], "hello");
        })
        .await;
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: env_lock serialises HOME mutations across the async test
    async fn execute_response_includes_decoded_output() {
        let _guard = env_lock();
        let cwd = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        write_template(cwd.path(), "demo.hcl", READ_TEMPLATE);
        write_config(home.path(), "");

        with_home(home.path(), || async {
            let app = build_router(WebState {
                cwd: cwd.path().to_path_buf(),
                bearer_token: TEST_TOKEN.to_string(),
            });

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/execute")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {TEST_TOKEN}"))
                        .body(Body::from(
                            serde_json::json!({
                                "command": "demo.echo",
                                "args": {"value": "hello"},
                                "confirm_write": false
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), 200);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let parsed: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(parsed["decoded"], "hello");
        })
        .await;
    }

    async fn with_home<F, Fut>(path: &Path, run: F)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let previous = std::env::var_os("HOME");
        // SAFETY: tests serialize HOME mutations with a global mutex.
        unsafe { std::env::set_var("HOME", path) };
        run().await;
        match previous {
            // SAFETY: tests serialize HOME mutations with a global mutex.
            Some(home) => unsafe { std::env::set_var("HOME", home) },
            // SAFETY: tests serialize HOME mutations with a global mutex.
            None => unsafe { std::env::remove_var("HOME") },
        }
    }

    fn write_template(cwd: &Path, name: &str, content: &str) {
        let template_dir = cwd.join("templates");
        fs::create_dir_all(&template_dir).unwrap();
        fs::write(template_dir.join(name), content).unwrap();
    }

    fn write_config(home: &Path, content: &str) {
        let config_dir = home.join(".config/earl");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("config.toml"), content).unwrap();
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }
}
