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
    fn deserialize_sandbox_config() {
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
    fn default_config_has_expected_sandbox_defaults() {
        let loaded: Config = toml::from_str("").unwrap();
        assert!(loaded.sandbox.sql_connection_allowlist.is_empty());
        assert!(loaded.sandbox.sql_force_read_only);
    }

    #[test]
    fn deserialize_ignores_unknown_legacy_table() {
        let loaded: Config = toml::from_str(
            r#"
[search]
top_k = 11
rerank_k = 9

[legacy_template_sources."acme/tools"]
url = "https://example.com/templates/github.hcl"
"#,
        )
        .unwrap();
        assert_eq!(loaded.search.top_k, 11);
        assert_eq!(loaded.search.rerank_k, 9);
    }

    #[test]
    fn deserialize_jwt_config() {
        let loaded: Config = toml::from_str(
            r#"
[auth.jwt]
audience = "https://api.example.com"
issuer = "https://accounts.example.com"
jwks_uri = "https://accounts.example.com/.well-known/jwks.json"
algorithms = ["RS256", "ES256"]
clock_skew_seconds = 60
"#,
        )
        .unwrap();

        let jwt = loaded.auth.jwt.expect("jwt config should be present");
        assert_eq!(jwt.audience, "https://api.example.com");
        assert_eq!(jwt.issuer.as_deref(), Some("https://accounts.example.com"));
        assert_eq!(
            jwt.jwks_uri.as_deref(),
            Some("https://accounts.example.com/.well-known/jwks.json")
        );
        assert_eq!(jwt.algorithms, vec!["RS256", "ES256"]);
        assert_eq!(jwt.clock_skew_seconds, 60);
        assert_eq!(jwt.jwks_cache_max_age_seconds, 900); // default
        assert!(jwt.oidc_discovery_url.is_none());
    }

    #[test]
    fn deserialize_jwt_config_defaults() {
        let loaded: Config = toml::from_str(
            r#"
[auth.jwt]
audience = "my-audience"
"#,
        )
        .unwrap();

        let jwt = loaded.auth.jwt.expect("jwt config should be present");
        assert_eq!(jwt.audience, "my-audience");
        assert_eq!(jwt.algorithms, vec!["RS256"]);
        assert_eq!(jwt.clock_skew_seconds, 30);
        assert_eq!(jwt.jwks_cache_max_age_seconds, 900);
    }

    #[test]
    fn deserialize_policy_rules() {
        let loaded: Config = toml::from_str(
            r#"
[[policy]]
subjects = ["user:alice", "group:admins"]
tools = ["github.*"]
effect = "allow"

[[policy]]
subjects = ["*"]
tools = ["github.delete_repo"]
modes = ["write"]
effect = "deny"
"#,
        )
        .unwrap();

        assert_eq!(loaded.policy.len(), 2);

        let rule0 = &loaded.policy[0];
        assert_eq!(rule0.subjects, vec!["user:alice", "group:admins"]);
        assert_eq!(rule0.tools, vec!["github.*"]);
        assert_eq!(rule0.effect, PolicyEffect::Allow);
        assert!(rule0.modes.is_none());

        let rule1 = &loaded.policy[1];
        assert_eq!(rule1.subjects, vec!["*"]);
        assert_eq!(rule1.tools, vec!["github.delete_repo"]);
        assert_eq!(rule1.effect, PolicyEffect::Deny);
        assert_eq!(rule1.modes.as_ref().unwrap(), &vec![PolicyMode::Write]);
    }

    #[test]
    fn default_config_has_no_jwt_and_empty_policies() {
        let loaded: Config = toml::from_str("").unwrap();
        assert!(loaded.auth.jwt.is_none());
        assert!(loaded.policy.is_empty());
    }
}
