use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::template::schema::AllowRule;

const APP_NAME: &str = "earl";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub sandbox: SandboxConfig,
    #[serde(default)]
    pub policy: Vec<PolicyRule>,
    #[serde(default)]
    pub environments: EnvironmentsConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentsConfig {
    pub default: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_rerank_k")]
    pub rerank_k: usize,
    #[serde(default)]
    pub local: LocalSearchConfig,
    #[serde(default)]
    pub remote: RemoteSearchConfig,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            top_k: default_top_k(),
            rerank_k: default_rerank_k(),
            local: LocalSearchConfig::default(),
            remote: RemoteSearchConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalSearchConfig {
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    #[serde(default = "default_reranker_model")]
    pub reranker_model: String,
}

impl Default for LocalSearchConfig {
    fn default() -> Self {
        Self {
            embedding_model: default_embedding_model(),
            reranker_model: default_reranker_model(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteSearchConfig {
    #[serde(default)]
    pub enabled: bool,
    pub base_url: Option<String>,
    pub api_key_secret: Option<String>,
    #[serde(default = "default_embeddings_path")]
    pub embeddings_path: String,
    #[serde(default = "default_rerank_path")]
    pub rerank_path: String,
    #[serde(default = "default_true")]
    pub openai_compatible: bool,
    #[serde(default = "default_remote_timeout_ms")]
    pub timeout_ms: u64,
}

impl Default for RemoteSearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: None,
            api_key_secret: None,
            embeddings_path: default_embeddings_path(),
            rerank_path: default_rerank_path(),
            openai_compatible: true,
            timeout_ms: default_remote_timeout_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub profiles: BTreeMap<String, OAuthProfile>,
    pub jwt: Option<JwtConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtConfig {
    pub oidc_discovery_url: Option<String>,
    pub issuer: Option<String>,
    pub jwks_uri: Option<String>,
    pub audience: String,
    #[serde(default = "default_algorithms")]
    pub algorithms: Vec<String>,
    #[serde(default = "default_clock_skew_seconds")]
    pub clock_skew_seconds: u64,
    #[serde(default = "default_jwks_cache_max_age_seconds")]
    pub jwks_cache_max_age_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct NetworkConfig {
    #[serde(default)]
    pub allow: Vec<AllowRule>,
    #[serde(default)]
    pub proxy_profiles: BTreeMap<String, ProxyProfile>,
    /// Allow requests to RFC 1918 private addresses and loopback interfaces.
    /// Disabled by default to prevent SSRF attacks. Enable only for trusted
    /// homelab or self-hosted service environments.
    #[serde(default)]
    pub allow_private_ips: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub bash_max_time_ms: Option<u64>,
    #[serde(default)]
    pub bash_max_output_bytes: Option<u64>,
    #[serde(default)]
    pub bash_allow_network: bool,
    #[serde(default)]
    pub bash_max_memory_bytes: Option<u64>,
    #[serde(default)]
    pub bash_max_cpu_time_ms: Option<u64>,
    #[serde(default = "default_true")]
    pub sql_force_read_only: bool,
    #[serde(default)]
    pub sql_max_rows: Option<u64>,
    #[serde(default)]
    pub sql_connection_allowlist: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            bash_max_time_ms: None,
            bash_max_output_bytes: None,
            bash_allow_network: false,
            bash_max_memory_bytes: None,
            bash_max_cpu_time_ms: None,
            sql_force_read_only: true,
            sql_max_rows: None,
            sql_connection_allowlist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyProfile {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OAuthProfile {
    pub flow: OAuthFlow,
    pub client_id: String,
    pub client_secret_key: Option<String>,
    pub issuer: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub device_authorization_url: Option<String>,
    pub redirect_url: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub use_auth_request_body: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthFlow {
    AuthCodePkce,
    DeviceCode,
    ClientCredentials,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolicyRule {
    pub subjects: Vec<String>,
    pub tools: Vec<String>,
    #[serde(default)]
    pub modes: Option<Vec<PolicyMode>>,
    pub effect: PolicyEffect,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicyMode {
    Read,
    Write,
}

pub fn load_config() -> Result<Config> {
    let path = config_file_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed reading config file {}", path.display()))?;
    let cfg: Config =
        toml::from_str(&raw).with_context(|| format!("invalid config TOML {}", path.display()))?;
    Ok(cfg)
}

pub fn ensure_runtime_dirs() -> Result<()> {
    for path in [
        config_dir(),
        state_dir(),
        cache_dir(),
        global_templates_dir(),
    ] {
        fs::create_dir_all(&path)
            .with_context(|| format!("failed creating directory {}", path.display()))?;
    }
    Ok(())
}

pub fn local_templates_dir(cwd: &Path) -> PathBuf {
    cwd.join("templates")
}

pub fn global_templates_dir() -> PathBuf {
    config_dir().join("templates")
}

pub fn config_dir() -> PathBuf {
    home_dir().join(".config").join(APP_NAME)
}

pub fn config_file_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn state_dir() -> PathBuf {
    home_dir().join(".local").join("state").join(APP_NAME)
}

pub fn cache_dir() -> PathBuf {
    home_dir().join(".cache").join(APP_NAME)
}

pub fn secrets_index_path() -> PathBuf {
    state_dir().join("secrets-index.json")
}

pub fn search_index_path() -> PathBuf {
    cache_dir().join("search-index-v1.json")
}

pub fn catalog_cache_path() -> PathBuf {
    cache_dir().join(format!(
        "catalog-{}.bin",
        crate::template::cache::CACHE_VERSION
    ))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()))
        .expect("could not determine home directory; ensure $HOME is set")
}

fn default_top_k() -> usize {
    40
}

fn default_rerank_k() -> usize {
    10
}

fn default_embedding_model() -> String {
    "BGESmallENV15Q".to_string()
}

fn default_reranker_model() -> String {
    "JINARerankerV1TurboEn".to_string()
}

fn default_true() -> bool {
    true
}

fn default_embeddings_path() -> String {
    "/embeddings".to_string()
}

fn default_rerank_path() -> String {
    "/rerank".to_string()
}

fn default_remote_timeout_ms() -> u64 {
    10_000
}

fn default_algorithms() -> Vec<String> {
    vec!["RS256".to_string()]
}

fn default_clock_skew_seconds() -> u64 {
    30
}

fn default_jwks_cache_max_age_seconds() -> u64 {
    900
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_connection_allowlist_parsed_from_toml() {
        let loaded: Config = toml::from_str(
            r#"
[sandbox]
sql_connection_allowlist = ["myapp.db_url"]
"#,
        )
        .unwrap();
        assert_eq!(
            loaded.sandbox.sql_connection_allowlist,
            vec!["myapp.db_url"]
        );
    }

    #[test]
    fn default_sandbox_sql_connection_allowlist_is_empty() {
        let loaded: Config = toml::from_str("").unwrap();
        assert!(loaded.sandbox.sql_connection_allowlist.is_empty());
    }

    #[test]
    fn default_sandbox_sql_force_read_only_is_true() {
        let loaded: Config = toml::from_str("").unwrap();
        assert!(loaded.sandbox.sql_force_read_only);
    }

    #[test]
    fn deserialize_ignores_unknown_legacy_table() {
        toml::from_str::<Config>(
            r#"
[legacy_template_sources."acme/tools"]
url = "https://example.com/templates/github.hcl"
"#,
        )
        .unwrap();
    }

    fn jwt_with_explicit_fields() -> JwtConfig {
        toml::from_str::<Config>(
            r#"
[auth.jwt]
audience = "https://api.example.com"
issuer = "https://accounts.example.com"
jwks_uri = "https://accounts.example.com/.well-known/jwks.json"
algorithms = ["RS256", "ES256"]
clock_skew_seconds = 60
"#,
        )
        .unwrap()
        .auth
        .jwt
        .expect("jwt config should be present")
    }

    #[test]
    fn jwt_config_audience_parsed_from_toml() {
        assert_eq!(
            jwt_with_explicit_fields().audience,
            "https://api.example.com"
        );
    }

    #[test]
    fn jwt_config_issuer_parsed_from_toml() {
        assert_eq!(
            jwt_with_explicit_fields().issuer.as_deref(),
            Some("https://accounts.example.com")
        );
    }

    #[test]
    fn jwt_config_jwks_uri_parsed_from_toml() {
        assert_eq!(
            jwt_with_explicit_fields().jwks_uri.as_deref(),
            Some("https://accounts.example.com/.well-known/jwks.json")
        );
    }

    #[test]
    fn jwt_config_explicit_algorithms_parsed_from_toml() {
        assert_eq!(
            jwt_with_explicit_fields().algorithms,
            vec!["RS256", "ES256"]
        );
    }

    #[test]
    fn jwt_config_explicit_clock_skew_seconds_parsed_from_toml() {
        assert_eq!(jwt_with_explicit_fields().clock_skew_seconds, 60);
    }

    #[test]
    fn jwt_config_algorithms_defaults_to_rs256() {
        let loaded: Config = toml::from_str(
            r#"
[auth.jwt]
audience = "my-audience"
"#,
        )
        .unwrap();
        let jwt = loaded.auth.jwt.expect("jwt config should be present");
        assert_eq!(jwt.algorithms, vec!["RS256"]);
    }

    #[test]
    fn jwt_config_clock_skew_defaults_to_30_seconds() {
        let loaded: Config = toml::from_str(
            r#"
[auth.jwt]
audience = "my-audience"
"#,
        )
        .unwrap();
        let jwt = loaded.auth.jwt.expect("jwt config should be present");
        assert_eq!(jwt.clock_skew_seconds, 30);
    }

    #[test]
    fn jwt_config_jwks_cache_max_age_defaults_to_900_seconds() {
        let loaded: Config = toml::from_str(
            r#"
[auth.jwt]
audience = "my-audience"
"#,
        )
        .unwrap();
        let jwt = loaded.auth.jwt.expect("jwt config should be present");
        assert_eq!(jwt.jwks_cache_max_age_seconds, 900);
    }

    #[test]
    fn jwt_config_oidc_discovery_url_defaults_to_none() {
        let loaded: Config = toml::from_str(
            r#"
[auth.jwt]
audience = "my-audience"
"#,
        )
        .unwrap();
        let jwt = loaded.auth.jwt.expect("jwt config should be present");
        assert!(jwt.oidc_discovery_url.is_none());
    }

    fn allow_rule() -> PolicyRule {
        toml::from_str::<Config>(
            r#"
[[policy]]
subjects = ["user:alice", "group:admins"]
tools = ["github.*"]
effect = "allow"
"#,
        )
        .unwrap()
        .policy
        .into_iter()
        .next()
        .expect("policy should have one rule")
    }

    #[test]
    fn policy_allow_rule_subjects_parsed_from_toml() {
        assert_eq!(allow_rule().subjects, vec!["user:alice", "group:admins"]);
    }

    #[test]
    fn policy_allow_rule_tools_parsed_from_toml() {
        assert_eq!(allow_rule().tools, vec!["github.*"]);
    }

    #[test]
    fn policy_allow_rule_effect_is_allow() {
        assert_eq!(allow_rule().effect, PolicyEffect::Allow);
    }

    #[test]
    fn policy_allow_rule_has_no_modes() {
        assert!(allow_rule().modes.is_none());
    }

    fn deny_rule_with_mode() -> PolicyRule {
        toml::from_str::<Config>(
            r#"
[[policy]]
subjects = ["*"]
tools = ["github.delete_repo"]
modes = ["write"]
effect = "deny"
"#,
        )
        .unwrap()
        .policy
        .into_iter()
        .next()
        .expect("policy should have one rule")
    }

    #[test]
    fn policy_deny_rule_subjects_parsed_from_toml() {
        assert_eq!(deny_rule_with_mode().subjects, vec!["*"]);
    }

    #[test]
    fn policy_deny_rule_tools_parsed_from_toml() {
        assert_eq!(deny_rule_with_mode().tools, vec!["github.delete_repo"]);
    }

    #[test]
    fn policy_deny_rule_effect_is_deny() {
        assert_eq!(deny_rule_with_mode().effect, PolicyEffect::Deny);
    }

    #[test]
    fn policy_deny_rule_write_mode_filter_parsed_from_toml() {
        assert_eq!(
            deny_rule_with_mode().modes.as_ref().unwrap(),
            &vec![PolicyMode::Write]
        );
    }

    #[test]
    fn default_config_has_no_jwt_config() {
        let loaded: Config = toml::from_str("").unwrap();
        assert!(loaded.auth.jwt.is_none());
    }

    #[test]
    fn default_config_has_empty_policy_list() {
        let loaded: Config = toml::from_str("").unwrap();
        assert!(loaded.policy.is_empty());
    }

    #[test]
    fn environments_default_env_parsed_from_toml() {
        let cfg: Config = toml::from_str(
            r#"
[environments]
default = "staging"
"#,
        )
        .unwrap();
        assert_eq!(cfg.environments.default.as_deref(), Some("staging"));
    }

    #[test]
    fn default_environments_config_has_no_default() {
        let cfg: Config = toml::from_str("").unwrap();
        assert!(cfg.environments.default.is_none());
    }

    #[test]
    fn environments_config_rejects_unknown_field() {
        let result: Result<Config, _> = toml::from_str(
            r#"
[environments]
defaullt = "staging"
"#,
        );
        assert!(result.is_err());
    }
}
