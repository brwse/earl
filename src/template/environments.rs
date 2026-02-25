use anyhow::{Result, bail};

use super::schema::{CommandTemplate, EnvironmentOverride, OperationTemplate, ResultTemplate};

/// Resolves the active environment name from available sources in priority order:
/// 1. CLI `--env` flag
/// 2. `[environments] default` in config.toml
/// 3. `default` field in the template's `environments` block
/// 4. None (no active environment)
pub fn resolve_active_env<'a>(
    cli_env: Option<&'a str>,
    config_env: Option<&'a str>,
    template_default: Option<&'a str>,
) -> Option<&'a str> {
    cli_env.or(config_env).or(template_default)
}

/// Validates that an environment name contains only [a-zA-Z0-9_-] and is 1–64 chars.
pub fn validate_env_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        bail!("environment name must be 1–64 characters, got `{name}`");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!(
            "environment name `{name}` contains invalid characters; \
             only alphanumeric, underscore, and hyphen are allowed"
        );
    }
    Ok(())
}

/// Selects the operation and result template to use for the given environment name.
///
/// If `env_name` matches a per-command override, uses that operation (and its
/// result if provided, or the command default result otherwise).
/// If there is no matching override, uses the command defaults.
pub fn select_for_env<'a>(
    cmd: &'a CommandTemplate,
    env_name: Option<&str>,
) -> (&'a OperationTemplate, &'a ResultTemplate) {
    let Some(name) = env_name else {
        return (&cmd.operation, &cmd.result);
    };
    match cmd.environment_overrides.get(name) {
        Some(EnvironmentOverride {
            operation,
            result: Some(result),
        }) => (operation, result),
        Some(EnvironmentOverride {
            operation,
            result: None,
        }) => (operation, &cmd.result),
        None => (&cmd.operation, &cmd.result),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_prefers_cli_flag() {
        assert_eq!(
            resolve_active_env(Some("staging"), Some("prod"), Some("dev")),
            Some("staging")
        );
    }

    #[test]
    fn resolve_falls_back_to_config() {
        assert_eq!(
            resolve_active_env(None, Some("prod"), Some("dev")),
            Some("prod")
        );
    }

    #[test]
    fn resolve_falls_back_to_template() {
        assert_eq!(resolve_active_env(None, None, Some("dev")), Some("dev"));
    }

    #[test]
    fn resolve_returns_none_when_nothing_set() {
        assert_eq!(resolve_active_env(None, None, None), None);
    }

    #[test]
    fn alphanumeric_name_is_valid() {
        assert!(validate_env_name("production").is_ok());
    }

    #[test]
    fn name_with_hyphens_is_valid() {
        assert!(validate_env_name("staging-eu").is_ok());
    }

    #[test]
    fn name_with_underscores_and_digits_is_valid() {
        assert!(validate_env_name("dev_local_2").is_ok());
    }

    #[test]
    fn empty_name_returns_error() {
        assert!(validate_env_name("").is_err());
    }

    #[test]
    fn name_with_spaces_returns_error() {
        assert!(validate_env_name("has space").is_err());
    }

    #[test]
    fn name_with_path_traversal_returns_error() {
        assert!(validate_env_name("../etc/passwd").is_err());
    }

    #[test]
    fn name_exceeding_max_length_returns_error() {
        assert!(validate_env_name(&"a".repeat(65)).is_err());
    }
}
