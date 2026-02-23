use std::env;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::process::{Command as ProcessCommand, Stdio};

use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use comfy_table::{Cell, Table, presets::UTF8_FULL};
use tracing_subscriber::EnvFilter;

use crate::auth::oauth2::OAuthManager;
use crate::cli::{
    AuthSubcommand, Cli, Command, CompletionArgs, CompletionShell, DoctorArgs, McpArgs,
    McpMode as CliMcpMode, McpTransport, SecretsSubcommand, TemplateGenerateArgs,
    TemplateImportScope, TemplateMode, TemplateSubcommand, WebArgs,
};
use crate::config::{self, Config};
use crate::doctor::{self, DoctorReport, DoctorStatus};
use crate::expression::binder::bind_arguments;
use crate::expression::cli_args::parse_cli_args;
use crate::mcp::{self, McpMode, McpServerOptions, ServerTransport};
use crate::output::human::render_human_output;
use crate::output::json::render_json_output;
use crate::protocol::builder::build_prepared_request;
use crate::protocol::executor::execute_prepared_request;
use crate::search::service::search_templates;
use crate::secrets::SecretManager;
use crate::template::catalog::TemplateScope;
use crate::template::import::import_template_from_source_ref;
use crate::template::loader::{load_catalog, validate_all};
use crate::template::schema::{CommandMode, ParamSpec};
use crate::web;

pub async fn run(cli: Cli) -> Result<()> {
    init_tracing();
    config::ensure_runtime_dirs()?;

    match cli.command {
        Command::Call(args) => {
            let cfg = config::load_config()?;
            run_call(args.command, args.args, args.yes, args.json, cfg).await
        }
        Command::Templates(args) => {
            let cfg = config::load_config()?;
            run_templates(args.command, args.json, cfg).await
        }
        Command::Secrets(args) => run_secrets(args.command),
        Command::Auth(args) => {
            let cfg = config::load_config()?;
            run_auth(args.command, cfg).await
        }
        Command::Mcp(args) => {
            let cfg = config::load_config()?;
            run_mcp(args, cfg).await
        }
        Command::Doctor(args) => run_doctor(args),
        Command::Web(args) => run_web(args).await,
        Command::Completion(args) => run_completion(args),
    }
}

async fn run_call(
    command: String,
    raw_args: Vec<String>,
    auto_yes: bool,
    json_mode: bool,
    cfg: Config,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let catalog = load_catalog(&cwd)?;

    let entry = catalog
        .get(&command)
        .ok_or_else(|| anyhow::anyhow!("unknown command `{command}`"))?;

    let expr = parse_cli_args(&command, &raw_args, &entry.template.params)?;
    let bound_args = bind_arguments(&expr, &entry.template.params)?;

    if entry.mode == CommandMode::Write && !auto_yes {
        prompt_write_confirmation(&entry.key)?;
    }

    let secret_manager = SecretManager::new();
    let allow_rules = cfg.network.allow.clone();
    let proxy_profiles = cfg.network.proxy_profiles.clone();
    let sandbox_config = cfg.sandbox.clone();
    let oauth_manager = OAuthManager::new(cfg, SecretManager::new())?; // OAuthManager takes ownership, so a separate instance is needed

    let prepared = build_prepared_request(
        entry,
        bound_args,
        &secret_manager,
        &oauth_manager,
        &allow_rules,
        &proxy_profiles,
        &sandbox_config,
    )
    .await?;
    if prepared.stream {
        use crate::output::stream::render_streaming_output;
        use crate::protocol::executor::start_streaming_request;

        let result_template = prepared.result_template.clone();
        let args = prepared.args.clone();
        let redactor = prepared.redactor.clone();

        let (rx, handle) = start_streaming_request(prepared);
        render_streaming_output(rx, &result_template, &args, &redactor, json_mode).await?;
        // Wait for the producer to finish and propagate any errors
        handle.await??;
    } else {
        let execution = execute_prepared_request(&prepared).await?;
        if json_mode {
            let rendered = render_json_output(&execution);
            let redacted = prepared.redactor.redact_json(&rendered);
            println!("{}", serde_json::to_string_pretty(&redacted)?);
        } else {
            let output =
                render_human_output(&prepared.result_template, &prepared.args, &execution.result)?;
            println!("{}", prepared.redactor.redact(&output));
        }
    }

    Ok(())
}

async fn run_templates(command: TemplateSubcommand, json_mode: bool, cfg: Config) -> Result<()> {
    let cwd = env::current_dir()?;

    match command {
        TemplateSubcommand::List(args) => {
            let catalog = load_catalog(&cwd)?;
            let mut rows = Vec::new();
            for entry in catalog.values() {
                if let Some(category) = &args.category
                    && !entry.categories.iter().any(|c| c == category)
                {
                    continue;
                }

                if let Some(mode) = args.mode {
                    let expected = match mode {
                        TemplateMode::Read => CommandMode::Read,
                        TemplateMode::Write => CommandMode::Write,
                    };
                    if entry.mode != expected {
                        continue;
                    }
                }

                rows.push(TemplateListRow {
                    command: entry.key.clone(),
                    mode: entry.mode.as_str().to_string(),
                    categories: entry.categories.clone(),
                    title: entry.title.clone(),
                    summary: entry.summary.clone(),
                    description: entry.description.clone(),
                    input_schema: entry.template.params.clone(),
                    source: TemplateListSource {
                        path: entry.source.path.display().to_string(),
                        scope: template_scope_label(entry.source.scope).to_string(),
                    },
                });
            }

            rows.sort_by(|a, b| a.command.cmp(&b.command));

            if json_mode {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                let mut table = Table::new();
                table.load_preset(UTF8_FULL).set_header([
                    "Command",
                    "Mode",
                    "Categories",
                    "Title",
                    "Summary",
                    "Description",
                    "Input Schema",
                    "Source",
                ]);
                for row in rows {
                    table.add_row(vec![
                        Cell::new(row.command),
                        Cell::new(row.mode),
                        Cell::new(row.categories.join(",")),
                        Cell::new(row.title),
                        Cell::new(row.summary),
                        Cell::new(row.description),
                        Cell::new(format_input_schema_list(&row.input_schema)),
                        Cell::new(row.source.path),
                    ]);
                }
                println!("{table}");
            }
        }
        TemplateSubcommand::Search(args) => {
            let catalog = load_catalog(&cwd)?;
            let secrets = SecretManager::new();
            let results =
                search_templates(&args.query, &catalog, &cfg, &secrets, args.limit).await?;

            if json_mode {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .set_header(["Score", "Command", "Summary"]);
                for hit in results {
                    table.add_row(vec![
                        Cell::new(format!("{:.4}", hit.score)),
                        Cell::new(hit.key),
                        Cell::new(hit.summary),
                    ]);
                }

                println!("{table}");
            }
        }
        TemplateSubcommand::Validate => {
            let files = validate_all(&cwd)?;
            if json_mode {
                let rendered_files: Vec<String> = files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();
                println!("{}", serde_json::to_string_pretty(&rendered_files)?);
            } else if files.is_empty() {
                println!("No templates found.");
            } else {
                for path in files {
                    println!("ok: {}", path.display());
                }
            }
        }
        TemplateSubcommand::Import(args) => {
            let destination_dir = match args.scope {
                TemplateImportScope::Local => config::local_templates_dir(&cwd),
                TemplateImportScope::Global => config::global_templates_dir(),
            };
            let imported =
                import_template_from_source_ref(&cwd, &args.source_ref, &destination_dir).await?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&imported)?);
            } else {
                println!(
                    "Imported template from `{}` to `{}`.",
                    imported.source_ref, imported.destination
                );
                if imported.required_credential_names.is_empty() {
                    println!("No required secrets were declared by this template.");
                } else {
                    println!("Required secrets:");
                    for name in &imported.required_credential_names {
                        println!("- {name}");
                    }
                    println!("Set up with:");
                    for name in &imported.required_credential_names {
                        println!("earl secrets set {name}");
                    }
                }
            }
        }
        TemplateSubcommand::Generate(args) => {
            run_template_generate(args, json_mode)?;
        }
    }

    Ok(())
}

fn run_secrets(command: SecretsSubcommand) -> Result<()> {
    let manager = SecretManager::new();

    match command {
        SecretsSubcommand::Set(args) => {
            let value = if args.stdin {
                let mut raw = String::new();
                io::stdin().read_to_string(&mut raw)?;
                raw.trim_end_matches(['\r', '\n']).to_string()
            } else {
                rpassword::prompt_password("Enter secret value: ")
                    .context("failed reading secret from terminal")?
            };

            manager.set(&args.key, secrecy::SecretString::new(value.into()))?;
            println!("Stored secret `{}`.", args.key);
        }
        SecretsSubcommand::Get(args) => {
            if let Some(meta) = manager.get(&args.key)? {
                println!("key: {}", args.key);
                println!("created_at: {}", meta.created_at.to_rfc3339());
                println!("updated_at: {}", meta.updated_at.to_rfc3339());
                println!("value: [REDACTED]");
            } else {
                bail!("secret `{}` not found", args.key);
            }
        }
        SecretsSubcommand::List => {
            let entries = manager.list()?;
            if entries.is_empty() {
                println!("No secrets found.");
            } else {
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL)
                    .set_header(["Key", "Created", "Updated"]);
                for meta in entries {
                    table.add_row(vec![
                        Cell::new(meta.key),
                        Cell::new(meta.created_at.to_rfc3339()),
                        Cell::new(meta.updated_at.to_rfc3339()),
                    ]);
                }
                println!("{table}");
            }
        }
        SecretsSubcommand::Delete(args) => {
            let deleted = manager.delete(&args.key)?;
            if deleted {
                println!("Deleted secret `{}`.", args.key);
            } else {
                println!("Secret `{}` did not exist.", args.key);
            }
        }
    }

    Ok(())
}

async fn run_auth(command: AuthSubcommand, cfg: Config) -> Result<()> {
    let oauth = OAuthManager::new(cfg, SecretManager::new())?;

    match command {
        AuthSubcommand::Login(args) => {
            oauth.login(&args.profile).await?;
            println!("Authenticated profile `{}`.", args.profile);
        }
        AuthSubcommand::Status(args) => {
            let status = oauth.status(&args.profile)?;
            println!("profile: {}", args.profile);
            println!("logged_in: {}", status.logged_in);
            if let Some(exp) = status.expires_at {
                println!("expires_at: {}", exp.to_rfc3339());
            }
            if !status.scopes.is_empty() {
                println!("scopes: {}", status.scopes.join(" "));
            }
        }
        AuthSubcommand::Refresh(args) => {
            oauth.refresh(&args.profile).await?;
            println!("Refreshed profile `{}`.", args.profile);
        }
        AuthSubcommand::Logout(args) => {
            let deleted = oauth.logout(&args.profile)?;
            if deleted {
                println!("Logged out profile `{}`.", args.profile);
            } else {
                println!("No token found for profile `{}`.", args.profile);
            }
        }
    }

    Ok(())
}

async fn run_mcp(args: McpArgs, cfg: Config) -> Result<()> {
    let cwd = env::current_dir()?;
    let catalog = load_catalog(&cwd)?;
    let transport = match args.transport {
        McpTransport::Stdio => ServerTransport::Stdio,
        McpTransport::Http => ServerTransport::Http,
    };
    let mode = match args.mode {
        CliMcpMode::Full => McpMode::Full,
        CliMcpMode::Discovery => McpMode::Discovery,
    };

    let options = McpServerOptions {
        transport,
        listen: args.listen,
        mode,
        auto_yes: args.yes,
        allow_unauthenticated: args.allow_unauthenticated,
    };

    mcp::run_server(options, catalog, cfg).await
}

fn run_doctor(args: DoctorArgs) -> Result<()> {
    let cwd = env::current_dir()?;
    let report = doctor::run_checks(&cwd);
    let summary = report.summary();

    if args.json {
        let payload = serde_json::json!({
            "checks": report.checks,
            "summary": summary,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        render_doctor_report(&report, &cwd.display().to_string());
    }

    if summary.error > 0 {
        bail!("doctor found {} error(s)", summary.error);
    }

    Ok(())
}

fn run_completion(args: CompletionArgs) -> Result<()> {
    let shell = match args.shell {
        CompletionShell::Bash => Shell::Bash,
        CompletionShell::Zsh => Shell::Zsh,
        CompletionShell::Fish => Shell::Fish,
        CompletionShell::PowerShell => Shell::PowerShell,
        CompletionShell::Elvish => Shell::Elvish,
    };

    let mut command = Cli::command();
    let bin_name = command.get_name().to_string();
    generate(shell, &mut command, bin_name, &mut io::stdout());
    Ok(())
}

async fn run_web(args: WebArgs) -> Result<()> {
    let cwd = env::current_dir()?;
    let listener = tokio::net::TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("failed to bind web listener on {}", args.listen))?;
    let bound = listener
        .local_addr()
        .context("failed reading bound web listener address")?;
    let url = web_url_for_addr(bound);

    let bearer_token = web::generate_bearer_token();
    println!("earl web playground listening at {url}");
    println!("Authorization: Bearer {bearer_token}");
    if !args.no_open {
        let _ = webbrowser::open(&url);
    }

    let router = web::build_router(web::WebState { cwd, bearer_token });
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .context("web server exited unexpectedly")
}

fn web_url_for_addr(addr: SocketAddr) -> String {
    if addr.is_ipv6() {
        format!("http://[{}]:{}", addr.ip(), addr.port())
    } else {
        format!("http://{}:{}", addr.ip(), addr.port())
    }
}

fn prompt_write_confirmation(command: &str) -> Result<()> {
    println!("Command `{command}` is write-enabled.");
    let answer = prompt("Type YES to continue: ")?;
    if answer.trim() != "YES" {
        bail!("write command aborted");
    }
    Ok(())
}

#[derive(Debug)]
struct TemplateGenerateInput {
    request: String,
}

fn run_template_generate(args: TemplateGenerateArgs, json_mode: bool) -> Result<()> {
    if json_mode {
        bail!("`earl templates generate` does not support --json");
    }

    println!("Template generation wizard");
    let request =
        prompt_required("Describe the template you want (include provider.command if known): ")?;

    let generate_input = TemplateGenerateInput {
        request: request.clone(),
    };
    let generated_prompt = build_template_generation_prompt(&generate_input);

    let (program, cli_args) = args
        .command
        .split_first()
        .context("missing coding CLI command")?;
    println!("Sending prompt to coding CLI: {}", args.command.join(" "));

    let mut child = ProcessCommand::new(program)
        .args(cli_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to start coding CLI `{program}`"))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .context("failed opening coding CLI stdin")?;
        stdin
            .write_all(generated_prompt.as_bytes())
            .context("failed writing prompt to coding CLI stdin")?;
        stdin
            .write_all(b"\n")
            .context("failed writing newline to coding CLI stdin")?;
    }

    let status = child
        .wait()
        .context("failed waiting for coding CLI process")?;
    if !status.success() {
        bail!("coding CLI exited with status {status}");
    }

    println!("Template request sent.");
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct TemplateListRow {
    command: String,
    mode: String,
    categories: Vec<String>,
    title: String,
    summary: String,
    description: String,
    input_schema: Vec<ParamSpec>,
    source: TemplateListSource,
}

#[derive(Debug, serde::Serialize)]
struct TemplateListSource {
    path: String,
    scope: String,
}

fn format_input_schema_list(params: &[ParamSpec]) -> String {
    if params.is_empty() {
        return "[]".to_string();
    }

    params
        .iter()
        .map(format_param_schema_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_param_schema_line(param: &ParamSpec) -> String {
    let mut details = Vec::new();
    if param.required {
        details.push("required".to_string());
    } else {
        details.push("optional".to_string());
    }

    if let Some(default) = &param.default {
        details.push(format!("default={default}"));
    }

    if let Some(description) = &param.description {
        let trimmed = description.trim();
        if !trimmed.is_empty() {
            details.push(trimmed.to_string());
        }
    }

    format!(
        "- {}: {} ({})",
        param.name,
        param.r#type,
        details.join(", ")
    )
}

fn template_scope_label(scope: TemplateScope) -> &'static str {
    match scope {
        TemplateScope::Local => "local",
        TemplateScope::Global => "global",
    }
}

fn prompt_required(label: &str) -> Result<String> {
    loop {
        let value = prompt(label)?;
        if !value.trim().is_empty() {
            return Ok(value);
        }
        println!("A value is required.");
    }
}

fn build_template_generation_prompt(input: &TemplateGenerateInput) -> String {
    let command_hint = extract_command_hint(&input.request);
    let hint_block = if let Some((provider, command)) = command_hint {
        format!(
            "- likely command key: `{provider}.{command}`\n- likely file: `templates/{provider}.hcl`"
        )
    } else {
        "- infer command key and file path from the user request".to_string()
    };

    format!(
        r#"You are editing templates for the earl CLI project.

User request:
{request}

Hints:
{hint_block}

Constraints:
- Do most of the design work yourself; avoid asking follow-up questions unless strictly necessary.
- Follow the earl template schema (`version`, `provider`, `commands`, `params`, `operation`, `result`).
- Create or update the command in the provider file (prefer `templates/<provider>.hcl`).
- Determine `annotations.mode` (`read` vs `write`) from the requested behavior.
- Keep existing commands in the same file unless they must be adjusted for schema correctness.
- Include a useful `title`, `summary`, and markdown `description`.
- Add a `## Guidance for AI agents` section and an `earl call provider.command --param value` usage example in the description.
- Model parameters in `params` with appropriate `type`, `required`, and `default`.
- Prefer least-privilege handling for secrets/auth and avoid overbroad network behavior.

After editing:
1. Run `earl templates validate`.
2. If validation fails, fix the template and rerun validation.
3. Return a brief summary of what changed and any assumptions.
"#,
        request = input.request,
        hint_block = hint_block,
    )
}

fn extract_command_hint(raw: &str) -> Option<(String, String)> {
    for token in raw.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                ',' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\''
            )
    }) {
        let Some((provider, command)) = token.split_once('.') else {
            continue;
        };
        if is_identifier(provider) && is_identifier(command) {
            return Some((provider.to_string(), command.to_string()));
        }
    }

    None
}

fn is_identifier(raw: &str) -> bool {
    !raw.is_empty()
        && raw
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn render_doctor_report(report: &DoctorReport, cwd: &str) {
    println!("Doctor checks for {cwd}");
    for check in &report.checks {
        println!(
            "[{}] {}: {}",
            doctor_status_label(check.status),
            check.id,
            check.message
        );
        if let Some(suggestion) = &check.suggestion {
            println!("  fix: {suggestion}");
        }
    }

    let summary = report.summary();
    println!(
        "Summary: {} ok, {} warning, {} error",
        summary.ok, summary.warning, summary.error
    );
}

fn doctor_status_label(status: DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Ok => "OK",
        DoctorStatus::Warning => "WARN",
        DoctorStatus::Error => "ERROR",
    }
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush().context("failed flushing stdout")?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .try_init();
}
