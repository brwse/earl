use std::path::Path;

use earl::template::environments::{resolve_active_env, select_for_env};
use earl::template::parser::parse_template_hcl;
use earl::template::validator::validate_template_file;

fn load_fixture() -> earl::template::schema::TemplateFile {
    let content = include_str!("fixtures/environments_test.hcl");
    parse_template_hcl(content, Path::new("tests/fixtures")).unwrap()
}

#[test]
fn fixture_parses_without_error() {
    load_fixture();
}

#[test]
fn fixture_validates_without_error() {
    let file = load_fixture();
    validate_template_file(&file).unwrap();
}

#[test]
fn production_environment_is_present() {
    let file = load_fixture();
    let envs = file.environments.as_ref().unwrap();
    assert!(envs.environments.contains_key("production"));
}

#[test]
fn staging_environment_is_present() {
    let file = load_fixture();
    let envs = file.environments.as_ref().unwrap();
    assert!(envs.environments.contains_key("staging"));
}

#[test]
fn fixture_default_environment_is_production() {
    let file = load_fixture();
    let envs = file.environments.as_ref().unwrap();
    assert_eq!(envs.default.as_deref(), Some("production"));
}

#[test]
fn production_base_url_is_https_prod_example_com() {
    let file = load_fixture();
    let envs = file.environments.as_ref().unwrap();
    assert_eq!(
        envs.environments["production"]["base_url"],
        "https://prod.example.com"
    );
}

#[test]
fn staging_base_url_is_https_staging_example_com() {
    let file = load_fixture();
    let envs = file.environments.as_ref().unwrap();
    assert_eq!(
        envs.environments["staging"]["base_url"],
        "https://staging.example.com"
    );
}

#[test]
#[cfg(feature = "http")]
fn select_uses_default_operation_when_no_env() {
    let file = load_fixture();
    let cmd = file.commands.get("override_in_staging").unwrap();
    let (op, _) = select_for_env(cmd, None);
    // Default operation is HTTP GET
    assert!(matches!(
        op,
        earl::template::schema::OperationTemplate::Http(_)
    ));
}

#[test]
#[cfg(feature = "bash")]
fn select_uses_override_for_matching_env() {
    let file = load_fixture();
    let cmd = file.commands.get("override_in_staging").unwrap();
    let (op, _) = select_for_env(cmd, Some("staging"));
    assert!(matches!(
        op,
        earl::template::schema::OperationTemplate::Bash(_)
    ));
}

#[test]
#[cfg(feature = "bash")]
fn staging_override_script_is_echo_staging_override() {
    let file = load_fixture();
    let cmd = file.commands.get("override_in_staging").unwrap();
    let (op, _) = select_for_env(cmd, Some("staging"));
    assert_eq!(op.bash_script().unwrap(), "echo staging_override");
}

#[test]
#[cfg(feature = "http")]
fn select_falls_back_to_default_for_unrecognized_env() {
    let file = load_fixture();
    let cmd = file.commands.get("override_in_staging").unwrap();
    // "development" has no override — should fall back to default (HTTP GET)
    let (op, _) = select_for_env(cmd, Some("development"));
    assert!(matches!(
        op,
        earl::template::schema::OperationTemplate::Http(_)
    ));
}

#[test]
fn cli_arg_takes_priority_over_config_and_template_default() {
    assert_eq!(
        resolve_active_env(Some("cli"), Some("config"), Some("template")),
        Some("cli")
    );
}

#[test]
fn config_used_when_no_cli_arg() {
    assert_eq!(
        resolve_active_env(None, Some("config"), Some("template")),
        Some("config")
    );
}

#[test]
fn template_default_used_when_no_cli_or_config() {
    assert_eq!(
        resolve_active_env(None, None, Some("template")),
        Some("template")
    );
}

#[test]
fn resolve_returns_none_when_all_sources_absent() {
    assert_eq!(resolve_active_env(None::<&str>, None, None), None);
}

#[test]
#[cfg(feature = "bash")]
fn command_without_overrides_returns_default_for_production_env() {
    let file = load_fixture();
    let cmd = file.commands.get("echo_env").unwrap();
    let (op_prod, _) = select_for_env(cmd, Some("production"));
    assert_eq!(op_prod.bash_script().unwrap(), "echo {{ vars.label }}");
}

#[test]
#[cfg(feature = "bash")]
fn command_without_overrides_returns_default_for_staging_env() {
    let file = load_fixture();
    let cmd = file.commands.get("echo_env").unwrap();
    let (op_stg, _) = select_for_env(cmd, Some("staging"));
    assert_eq!(op_stg.bash_script().unwrap(), "echo {{ vars.label }}");
}
