use std::path::Path;

use anyhow::{Context, Result, bail};
use base64::Engine;
use serde_json::{Map, Value};

use super::schema::TemplateFile;

/// Parses an HCL template file into the canonical schema struct.
///
/// `base_dir` is the directory containing the HCL file, used to resolve
/// `file("...")` references to external files.
pub fn parse_template_hcl(content: &str, base_dir: &Path) -> Result<TemplateFile> {
    let root_value: Value = hcl::from_str(content).context("invalid HCL")?;

    let mut root = match root_value {
        Value::Object(object) => object,
        _ => bail!("template root must be an object"),
    };

    normalize_environments_block(&mut root)?;
    let commands = normalize_commands(&mut root)?;
    root.insert("commands".to_string(), Value::Object(commands));

    let mut root_value = Value::Object(root);
    resolve_functions(&mut root_value, base_dir)?;

    serde_json::from_value(root_value).context("template does not match schema")
}

fn normalize_commands(root: &mut Map<String, Value>) -> Result<Map<String, Value>> {
    let commands_attr = root.remove("commands");
    let command_blocks = root.remove("command");

    if commands_attr.is_some() && command_blocks.is_some() {
        bail!("template must define either `commands` or `command`, not both");
    }

    match (commands_attr, command_blocks) {
        (Some(value), None) => normalize_command_map(value, "commands"),
        (None, Some(value)) => normalize_command_map(value, "command"),
        (None, None) => Ok(Map::new()),
        (Some(_), Some(_)) => unreachable!(),
    }
}

fn normalize_command_map(value: Value, field: &str) -> Result<Map<String, Value>> {
    let command_map = expect_object(value, field)?;
    let mut normalized = Map::new();

    for (command_name, command_value) in command_map {
        let command_path = format!("{field}.{command_name}");
        let mut command = expect_object(command_value, &command_path)?;
        normalize_params(&mut command, &command_path)?;
        normalize_environment_overrides(&mut command, &command_path)?;
        normalized.insert(command_name, Value::Object(command));
    }

    Ok(normalized)
}

fn normalize_params(command: &mut Map<String, Value>, command_path: &str) -> Result<()> {
    let params_attr = command.remove("params");
    let param_blocks = command.remove("param");

    if params_attr.is_some() && param_blocks.is_some() {
        bail!("{command_path} must define either `params` or `param`, not both");
    }

    if let Some(value) = params_attr {
        command.insert("params".to_string(), value);
        return Ok(());
    }

    let Some(param_blocks) = param_blocks else {
        return Ok(());
    };

    let params_path = format!("{command_path}.param");
    let param_map = expect_object(param_blocks, &params_path)?;

    let mut normalized = Vec::with_capacity(param_map.len());
    for (param_name, param_value) in param_map {
        let param_path = format!("{params_path}.{param_name}");
        let mut param = expect_object(param_value, &param_path)?;

        if let Some(name_value) = param.get("name") {
            match name_value {
                Value::String(existing_name) if existing_name == &param_name => {}
                Value::String(existing_name) => {
                    bail!(
                        "{param_path}.name (`{existing_name}`) must match parameter label `{param_name}`"
                    )
                }
                _ => bail!("{param_path}.name must be a string when provided"),
            }
        } else {
            param.insert("name".to_string(), Value::String(param_name));
        }

        normalized.push(Value::Object(param));
    }

    command.insert("params".to_string(), Value::Array(normalized));
    Ok(())
}

/// Extracts the provider-level `environments` block from the template root,
/// normalizing it from HCL's flat map into the canonical shape:
/// `{ default?, secrets?, environments: { name -> { key -> value } } }`
fn normalize_environments_block(root: &mut Map<String, Value>) -> Result<()> {
    let Some(env_value) = root.remove("environments") else {
        return Ok(());
    };
    let env_map = expect_object(env_value, "environments")?;

    let mut default: Option<Value> = None;
    let mut secrets: Value = Value::Array(vec![]);
    let mut named_envs = Map::new();

    for (key, val) in env_map {
        match key.as_str() {
            "default" => default = Some(val),
            "secrets" => secrets = val,
            _ => {
                let env_obj = expect_object(val, &format!("environments.{key}"))?;
                named_envs.insert(key, Value::Object(env_obj));
            }
        }
    }

    let mut normalized = Map::new();
    if let Some(d) = default {
        normalized.insert("default".to_string(), d);
    }
    normalized.insert("secrets".to_string(), secrets);
    normalized.insert("environments".to_string(), Value::Object(named_envs));

    root.insert("environments".to_string(), Value::Object(normalized));
    Ok(())
}

/// Extracts `environment "name" { ... }` blocks from a command object and
/// renames them to `environment_overrides` so serde can deserialize them.
fn normalize_environment_overrides(
    command: &mut Map<String, Value>,
    command_path: &str,
) -> Result<()> {
    let Some(env_blocks) = command.remove("environment") else {
        return Ok(());
    };
    let env_map = expect_object(env_blocks, &format!("{command_path}.environment"))?;
    command.insert("environment_overrides".to_string(), Value::Object(env_map));
    Ok(())
}

fn expect_object(value: Value, field: &str) -> Result<Map<String, Value>> {
    match value {
        Value::Object(object) => Ok(object),
        _ => bail!("{field} must be an object"),
    }
}

/// Walks a JSON value tree and evaluates HCL function expressions in string
/// values. Supported functions: `file("path")`, `base64encode(...)`,
/// `trimspace(...)`. Functions can be composed, e.g.
/// `trimspace(file("query.sql"))`.
fn resolve_functions(value: &mut Value, base_dir: &Path) -> Result<()> {
    match value {
        Value::String(s) => {
            if let Some(expr) = parse_expr(s) {
                *s = eval_expr(&expr, base_dir)?;
            }
        }
        Value::Array(arr) => {
            for item in arr {
                resolve_functions(item, base_dir)?;
            }
        }
        Value::Object(map) => {
            for val in map.values_mut() {
                resolve_functions(val, base_dir)?;
            }
        }
        _ => {}
    }
    Ok(())
}

// ── HCL function expression parser & evaluator ───────────

#[derive(Debug, PartialEq)]
enum Expr<'a> {
    Literal(&'a str),
    Call { name: &'a str, arg: Box<Expr<'a>> },
}

/// Tries to parse the string as a function expression. Returns `None` if the
/// string is a plain value (not a function call).
///
/// Handles both forms:
/// - Native HCL expression: `${file("path")}` (produced by `hcl::from_str`
///   when the HCL source is `script = file("path")`)
/// - Quoted string: `file("path")` (when the HCL source is
///   `script = "file(\"path\")"`)
fn parse_expr(s: &str) -> Option<Expr<'_>> {
    let s = s.trim();
    // Unwrap the `${...}` wrapper produced by hcl-rs for native expressions.
    let s = s
        .strip_prefix("${")
        .and_then(|r| r.strip_suffix('}'))
        .map(|r| r.trim())
        .unwrap_or(s);
    // Must look like `name(...)` to be treated as a function call.
    let paren = s.find('(')?;
    let name = &s[..paren];
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    // The closing paren must be the last character.
    if !s.ends_with(')') {
        return None;
    }
    let inner = s[paren + 1..s.len() - 1].trim();
    let arg = parse_arg(inner)?;
    Some(Expr::Call {
        name,
        arg: Box::new(arg),
    })
}

/// Parses a function argument: either a quoted string literal or a nested
/// function call.
fn parse_arg(s: &str) -> Option<Expr<'_>> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        return Some(Expr::Literal(inner));
    }
    // Try nested function call.
    parse_expr(s)
}

fn eval_expr(expr: &Expr<'_>, base_dir: &Path) -> Result<String> {
    match expr {
        Expr::Literal(s) => Ok((*s).to_string()),
        Expr::Call { name, arg } => {
            let arg_val = eval_expr(arg, base_dir)?;
            match *name {
                "file" => {
                    let arg_path = std::path::Path::new(&arg_val);
                    if arg_path.is_absolute() {
                        bail!("file() path must be relative, got `{arg_val}`");
                    }
                    if arg_val.contains("..") {
                        bail!("file() path must not contain `..` segments, got `{arg_val}`");
                    }
                    let resolved = base_dir.join(&arg_val);
                    let canonical = resolved.canonicalize().with_context(|| {
                        format!("failed resolving file path referenced by file(\"{arg_val}\")")
                    })?;
                    let canonical_base = base_dir.canonicalize().with_context(|| {
                        "failed canonicalizing template base directory".to_string()
                    })?;
                    if !canonical.starts_with(&canonical_base) {
                        bail!("file(\"{arg_val}\") resolves outside the template directory");
                    }
                    std::fs::read_to_string(&canonical).with_context(|| {
                        format!("failed reading file referenced by file(\"{arg_val}\")")
                    })
                }
                "base64encode" => {
                    Ok(base64::engine::general_purpose::STANDARD.encode(arg_val.as_bytes()))
                }
                "trimspace" => Ok(arg_val.trim().to_string()),
                _ => bail!("unknown function `{name}`"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Expr, eval_expr, parse_expr, parse_template_hcl};

    fn dummy_dir() -> &'static Path {
        Path::new(".")
    }

    fn block_style_single_param_fixture() -> crate::template::schema::TemplateFile {
        let template = r#"
version = 1
provider = "demo"
categories = ["sample"]

command "ping" {
  title = "Ping"
  summary = "Execute a simple ping request"
  description = "Sends a basic ping request and returns the raw response body."

  annotations {
    mode = "read"
    secrets = []
  }

  param "value" {
    type = "string"
    required = false
    default = "hello"
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.example.com/ping"
    query = {
      q = "{{ args.value }}"
    }
  }

  result {
    decode = "text"
    output = "{{ result }}"
  }
}
"#;
        parse_template_hcl(template, dummy_dir()).unwrap()
    }

    #[test]
    fn block_style_param_block_produces_single_param() {
        let parsed = block_style_single_param_fixture();
        let ping = parsed.commands.get("ping").unwrap();
        assert_eq!(ping.params.len(), 1);
    }

    #[test]
    fn block_style_param_label_becomes_param_name() {
        let parsed = block_style_single_param_fixture();
        let ping = parsed.commands.get("ping").unwrap();
        assert_eq!(ping.params[0].name, "value");
    }

    #[test]
    fn rejects_commands_and_command_together() {
        let template = r#"
version = 1
provider = "demo"
commands = {}
command "ping" {}
"#;

        parse_template_hcl(template, dummy_dir()).unwrap_err();
    }

    #[test]
    fn rejects_params_and_param_together() {
        let template = r#"
version = 1
provider = "demo"

command "ping" {
  title = "Ping"
  summary = "Ping"
  description = "Ping"

  annotations {
    mode = "read"
    secrets = []
  }

  params = []

  param "value" {
    type = "string"
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.example.com/ping"
  }

  result {
    output = "ok"
  }
}
"#;

        parse_template_hcl(template, dummy_dir()).unwrap_err();
    }

    #[test]
    fn file_function_call_is_parsed() {
        assert_eq!(
            parse_expr(r#"file("foo/bar.js")"#),
            Some(Expr::Call {
                name: "file",
                arg: Box::new(Expr::Literal("foo/bar.js")),
            })
        );
    }

    #[test]
    fn base64encode_function_call_is_parsed() {
        assert_eq!(
            parse_expr(r#"base64encode("hello")"#),
            Some(Expr::Call {
                name: "base64encode",
                arg: Box::new(Expr::Literal("hello")),
            })
        );
    }

    #[test]
    fn function_call_with_extra_whitespace_is_parsed() {
        assert_eq!(
            parse_expr(r#"  file( "script.sql" )  "#),
            Some(Expr::Call {
                name: "file",
                arg: Box::new(Expr::Literal("script.sql")),
            })
        );
    }

    #[test]
    fn nested_function_composition_is_parsed() {
        assert_eq!(
            parse_expr(r#"trimspace(file("query.sql"))"#),
            Some(Expr::Call {
                name: "trimspace",
                arg: Box::new(Expr::Call {
                    name: "file",
                    arg: Box::new(Expr::Literal("query.sql")),
                }),
            })
        );
    }

    #[test]
    fn native_hcl_expression_wrapper_stripped_for_simple_call() {
        assert_eq!(
            parse_expr(r#"${file("foo.js")}"#),
            Some(Expr::Call {
                name: "file",
                arg: Box::new(Expr::Literal("foo.js")),
            })
        );
    }

    #[test]
    fn native_hcl_expression_wrapper_stripped_for_nested_call() {
        assert_eq!(
            parse_expr(r#"${trimspace(file("query.sql"))}"#),
            Some(Expr::Call {
                name: "trimspace",
                arg: Box::new(Expr::Call {
                    name: "file",
                    arg: Box::new(Expr::Literal("query.sql")),
                }),
            })
        );
    }

    #[test]
    fn plain_string_without_parens_returns_none() {
        assert_eq!(parse_expr("not a file call"), None);
    }

    #[test]
    fn function_call_with_trailing_content_returns_none() {
        assert_eq!(parse_expr(r#"file("a.js") extra"#), None);
    }

    #[test]
    fn quoted_string_literal_returns_none() {
        assert_eq!(parse_expr(r#""just a string""#), None);
    }

    #[test]
    fn trimspace_strips_surrounding_whitespace() {
        let expr = parse_expr(r#"trimspace("  hello  ")"#).unwrap();
        assert_eq!(eval_expr(&expr, dummy_dir()).unwrap(), "hello");
    }

    #[test]
    fn base64encode_encodes_value_as_base64_string() {
        let expr = parse_expr(r#"base64encode("hello")"#).unwrap();
        assert_eq!(eval_expr(&expr, dummy_dir()).unwrap(), "aGVsbG8=");
    }

    #[test]
    fn unknown_function_name_returns_error() {
        let expr = parse_expr(r#"unknown("arg")"#).unwrap();
        eval_expr(&expr, dummy_dir()).unwrap_err();
    }

    fn parse_environments_fixture() -> crate::template::schema::ProviderEnvironments {
        let template = r#"
version = 1
provider = "demo"

environments {
  default = "production"
  secrets = ["demo.prod_key"]
  production {
    base_url = "https://api.demo.com"
  }
  staging {
    base_url = "https://api.staging.demo.com"
  }
}

command "ping" {
  title = "Ping"
  summary = "Ping"
  description = "Ping"
  annotations {
    mode = "read"
    secrets = []
  }
  operation {
    protocol = "http"
    method = "GET"
    url = "{{ vars.base_url }}/ping"
  }
  result {
    output = "ok"
  }
}
"#;
        let parsed = parse_template_hcl(template, dummy_dir()).unwrap();
        parsed.environments.expect("environments should be present")
    }

    #[test]
    fn environments_block_default_is_parsed() {
        let envs = parse_environments_fixture();
        assert_eq!(envs.default.as_deref(), Some("production"));
    }

    #[test]
    fn environments_block_secrets_are_parsed() {
        let envs = parse_environments_fixture();
        assert_eq!(envs.secrets, vec!["demo.prod_key"]);
    }

    #[test]
    fn environments_block_production_environment_is_parsed() {
        let envs = parse_environments_fixture();
        assert_eq!(
            envs.environments["production"]["base_url"],
            "https://api.demo.com"
        );
    }

    #[test]
    fn environments_block_staging_environment_is_parsed() {
        let envs = parse_environments_fixture();
        assert_eq!(
            envs.environments["staging"]["base_url"],
            "https://api.staging.demo.com"
        );
    }

    #[test]
    fn command_environment_block_is_normalized_to_environment_overrides() {
        let template = r#"
version = 1
provider = "demo"

command "ping" {
  title = "Ping"
  summary = "Ping"
  description = "Ping"
  annotations {
    mode = "read"
    secrets = []
  }
  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.demo.com/ping"
  }
  environment "staging" {
    operation {
      protocol = "bash"
      bash {
        script = "echo pong"
      }
    }
  }
  result {
    output = "ok"
  }
}
"#;
        let parsed = parse_template_hcl(template, dummy_dir()).unwrap();
        let cmd = parsed.commands.get("ping").unwrap();
        let override_ = cmd
            .environment_overrides
            .get("staging")
            .expect("staging override");
        assert!(matches!(
            override_.operation,
            crate::template::schema::OperationTemplate::Bash(_)
        ));
    }
}
