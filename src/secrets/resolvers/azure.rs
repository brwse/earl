use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use secrecy::SecretString;
use serde::Deserialize;

use crate::secrets::resolver::SecretResolver;
use crate::secrets::resolvers::{
    CachedToken, ERROR_BODY_MAX_LEN, truncate_body, validate_azure_vault_name,
    validate_path_segment,
};

/// A parsed `az://vault-name/secret-name` reference.
#[derive(Debug)]
struct AzureReference {
    vault_name: String,
    secret_name: String,
}

impl AzureReference {
    fn parse(reference: &str) -> Result<Self> {
        let after_scheme = reference
            .strip_prefix("az://")
            .ok_or_else(|| anyhow!("invalid Azure reference: must start with az://"))?;

        if after_scheme.is_empty() {
            bail!(
                "invalid Azure reference: vault name and secret name are required in {reference}"
            );
        }

        let segments: Vec<&str> = after_scheme.split('/').collect();

        // Reject empty segments from double slashes or trailing slashes —
        // e.g., `az://vault//secret` could silently misresolve.
        if segments.iter().any(|s| s.is_empty()) {
            bail!(
                "invalid Azure reference: contains empty path segments \
                 (double slash or trailing slash) in {reference}"
            );
        }

        match segments.len() {
            0 | 1 => {
                bail!(
                    "invalid Azure reference: expected az://vault-name/secret-name, got: {reference}"
                );
            }
            2 => {
                let vault_name = segments[0].to_string();
                let secret_name = segments[1].to_string();
                validate_azure_vault_name(&vault_name)?;
                validate_path_segment(&secret_name, "secret name")?;
                Ok(Self {
                    vault_name,
                    secret_name,
                })
            }
            _ => {
                bail!(
                    "invalid Azure reference: too many path segments, expected az://vault-name/secret-name, got: {reference}"
                );
            }
        }
    }
}

/// Resolver for Azure Key Vault secrets using the `az://` URI scheme.
///
/// Authentication is attempted in this order:
///
/// 1. **Client credentials (service principal)** — when `AZURE_TENANT_ID`,
///    `AZURE_CLIENT_ID`, and `AZURE_CLIENT_SECRET` are all set.
/// 2. **Managed Identity** — first via `IDENTITY_ENDPOINT`/`IDENTITY_HEADER`
///    (App Service, Functions, Container Apps), then via IMDS at
///    `169.254.169.254` (VMs, VMSS, AKS). Set `AZURE_CLIENT_ID` (without
///    `AZURE_CLIENT_SECRET`) for user-assigned identity.
/// 3. **Azure CLI** — `az account get-access-token` for developer workstations.
///
/// **Sovereign clouds:** Set `AZURE_AUTHORITY_HOST` (default: `login.microsoftonline.com`)
/// and `AZURE_VAULT_SUFFIX` (default: `vault.azure.net`) for non-public clouds
/// (e.g., `login.chinacloudapi.cn` / `vault.azure.cn` for Azure China).
///
/// References use the format `az://vault-name/secret-name`, where:
/// * `vault-name` is the Azure Key Vault name
/// * `secret-name` is the secret identifier within the vault
///
/// Example: `az://my-vault/api-key`
pub struct AzureResolver {
    token_cache: Mutex<Option<CachedToken>>,
}

impl AzureResolver {
    pub fn new() -> Self {
        Self {
            token_cache: Mutex::new(None),
        }
    }
}

impl Default for AzureResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Read an env var with a fallback default.
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Validate that a value is safe to use as a hostname component.
/// Rejects characters that could enable host-header injection or URL manipulation.
fn validate_hostname(value: &str, field_name: &str) -> Result<()> {
    if value.is_empty() {
        bail!("{field_name} must not be empty");
    }
    for ch in value.chars() {
        if ch == '/' || ch == '?' || ch == '#' || ch == '@' || ch.is_whitespace() || ch.is_control()
        {
            bail!(
                "{field_name} contains invalid character '{}' — \
                 must be a valid hostname (no '/', '?', '#', '@', whitespace, or control characters)",
                ch.escape_debug()
            );
        }
    }
    Ok(())
}

/// Validate that a value looks like a UUID or at least contains only path-safe characters.
fn validate_azure_id(value: &str, field_name: &str) -> Result<()> {
    if value.is_empty() {
        bail!("{field_name} must not be empty");
    }
    for ch in value.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' {
            bail!(
                "{field_name} contains invalid character '{}' — \
                 expected a UUID (alphanumeric and hyphens only)",
                ch.escape_debug()
            );
        }
    }
    Ok(())
}

/// The vault host suffix, controlling public vs sovereign cloud.
fn vault_suffix() -> Result<String> {
    let suffix = env_or("AZURE_VAULT_SUFFIX", "vault.azure.net");
    validate_hostname(&suffix, "AZURE_VAULT_SUFFIX")?;
    Ok(suffix)
}

impl SecretResolver for AzureResolver {
    fn scheme(&self) -> &str {
        "az"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let az_ref = AzureReference::parse(reference)?;

        // Obtain access token with cache — hold lock across check+fetch to
        // avoid TOCTOU race where multiple threads each fetch a fresh token.
        let access_token = {
            let mut cache = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(token) = cache.as_ref().and_then(|c| c.get_if_valid()) {
                token.to_string()
            } else {
                let result =
                    obtain_access_token().context("failed to obtain Azure AD access token")?;
                // Use server-provided expiry with a safety margin, falling back
                // to 50 minutes if the server didn't report a lifetime.
                // Apply a 2-minute safety margin to the server-reported lifetime.
                // Combined with get_if_valid()'s additional 30-second margin, the
                // total effective safety buffer is 150 seconds before actual expiry.
                let ttl_secs = result
                    .expires_in_secs
                    .map(|s| s.saturating_sub(120))
                    .unwrap_or(50 * 60);
                *cache = Some(CachedToken {
                    token: result.token.clone(),
                    expires_at: Instant::now() + Duration::from_secs(ttl_secs),
                });
                result.token
            }
        };

        let suffix = vault_suffix()?;
        let base_url = format!("https://{}.{}", az_ref.vault_name, suffix);
        let mut url =
            reqwest::Url::parse(&base_url).context("failed to parse Azure Key Vault base URL")?;
        // Use proper path-segment joining to encode the secret name.
        url.path_segments_mut()
            .map_err(|()| anyhow!("invalid Azure Key Vault URL"))?
            .push("secrets")
            .push(&az_ref.secret_name);
        url.query_pairs_mut().append_pair("api-version", "7.4");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            // Never follow redirects — could leak the Authorization header.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("failed to build HTTP client for Azure Key Vault")?;

        let request = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .build()
            .context("failed to build Azure Key Vault request")?;

        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.execute(request))
        })
        .context("Azure Key Vault API request failed")?;

        let status = response.status();
        if !status.is_success() {
            let body = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(response.text())
            })
            .unwrap_or_default();
            bail!(
                "Azure Key Vault API returned HTTP {}: {}",
                status.as_u16(),
                truncate_body(&body, ERROR_BODY_MAX_LEN)
            );
        }

        let body: SecretResponse = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.json())
        })
        .context("failed to parse Azure Key Vault API response")?;

        Ok(SecretString::from(body.value))
    }
}

/// Response from the Azure Key Vault `GET /secrets/{secret-name}` endpoint.
#[derive(Deserialize)]
struct SecretResponse {
    value: String,
}

/// Token response from the Azure AD OAuth2 token endpoint.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    /// Token lifetime in seconds (typically 3600-5400).
    expires_in: Option<u64>,
}

/// Token with its server-reported lifetime.
struct TokenWithExpiry {
    token: String,
    /// Seconds until expiry as reported by the server, if available.
    expires_in_secs: Option<u64>,
}

/// Obtain an Azure AD access token.
///
/// Tries authentication methods in order:
/// 1. **Client credentials** — when `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, and
///    `AZURE_CLIENT_SECRET` are all set.
/// 2. **Managed Identity** — first via `IDENTITY_ENDPOINT`/`IDENTITY_HEADER`
///    (App Service, Functions, Container Apps), then via IMDS (VMs, VMSS, AKS).
/// 3. **Azure CLI** — `az account get-access-token` for developer workstations.
fn obtain_access_token() -> Result<TokenWithExpiry> {
    let tenant_id = std::env::var("AZURE_TENANT_ID")
        .ok()
        .filter(|v| !v.is_empty());
    let client_id = std::env::var("AZURE_CLIENT_ID")
        .ok()
        .filter(|v| !v.is_empty());
    let client_secret = std::env::var("AZURE_CLIENT_SECRET")
        .ok()
        .filter(|v| !v.is_empty());

    // Path 1: Client credentials (service principal)
    if let (Some(tenant_id), Some(client_id), Some(client_secret)) =
        (&tenant_id, &client_id, &client_secret)
    {
        validate_azure_id(client_id, "AZURE_CLIENT_ID")?;
        return token_via_client_credentials(tenant_id, client_id, client_secret);
    }

    // Path 2a: App Service / Functions / Container Apps managed identity
    // (IDENTITY_ENDPOINT + IDENTITY_HEADER env vars).
    let app_svc_err = match token_via_app_service_identity(client_id.as_deref()) {
        Ok(token) => return Ok(token),
        Err(e) => e,
    };

    // Path 2b: VM / VMSS / AKS managed identity (IMDS at 169.254.169.254).
    let imds_err = match token_via_imds(client_id.as_deref()) {
        Ok(token) => return Ok(token),
        Err(e) => e,
    };

    // Path 3: Azure CLI fallback (`az account get-access-token`)
    let cli_err = match token_via_az_cli() {
        Ok(token) => return Ok(token),
        Err(e) => e,
    };

    bail!(
        "Azure authentication failed. Configure one of:\n  \
         1. Service principal: set AZURE_TENANT_ID, AZURE_CLIENT_ID, and AZURE_CLIENT_SECRET\n  \
         2. Managed Identity: run on Azure infrastructure \
         (App Service: {app_svc_err:#}; IMDS: {imds_err:#})\n  \
         3. Azure CLI: run `az login` (CLI failed: {cli_err:#})\n\n\
         For sovereign clouds, also set AZURE_AUTHORITY_HOST and AZURE_VAULT_SUFFIX."
    );
}

/// Obtain a token via Azure AD client credentials (service principal) flow.
fn token_via_client_credentials(
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<TokenWithExpiry> {
    let authority = env_or("AZURE_AUTHORITY_HOST", "login.microsoftonline.com");
    validate_hostname(&authority, "AZURE_AUTHORITY_HOST")?;
    validate_azure_id(tenant_id, "AZURE_TENANT_ID")?;
    let suffix = vault_suffix()?;

    // Build token URL using reqwest::Url for structural safety.
    let base = format!("https://{authority}");
    let mut token_url =
        reqwest::Url::parse(&base).context("failed to parse AZURE_AUTHORITY_HOST as URL base")?;
    token_url
        .path_segments_mut()
        .map_err(|()| anyhow!("invalid Azure AD authority URL"))?
        .push(tenant_id)
        .push("oauth2")
        .push("v2.0")
        .push("token");
    let scope = format!("https://{suffix}/.default");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        // Never follow redirects — could leak client_secret in the POST body.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client for Azure AD token exchange")?;

    let request = client
        .post(token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("scope", scope.as_str()),
        ])
        .build()
        .context("failed to build Azure AD token request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("Azure AD token exchange request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.text())
        })
        .unwrap_or_default();
        bail!(
            "Azure AD token exchange returned HTTP {}: {}",
            status.as_u16(),
            truncate_body(&body, ERROR_BODY_MAX_LEN)
        );
    }

    let token_resp: TokenResponse =
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(response.json()))
            .context("failed to parse Azure AD token response")?;

    Ok(TokenWithExpiry {
        token: token_resp.access_token,
        expires_in_secs: token_resp.expires_in,
    })
}

/// Obtain a token via App Service / Functions / Container Apps managed identity.
///
/// These platforms set `IDENTITY_ENDPOINT` and `IDENTITY_HEADER` env vars.
/// Uses a short 5-second timeout.
fn token_via_app_service_identity(client_id: Option<&str>) -> Result<TokenWithExpiry> {
    let endpoint = std::env::var("IDENTITY_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("IDENTITY_ENDPOINT not set (not running on App Service)"))?;
    let header = std::env::var("IDENTITY_HEADER")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("IDENTITY_HEADER not set (not running on App Service)"))?;
    // Validate IDENTITY_HEADER contains only HTTP header-safe bytes.
    // reqwest calls HeaderValue::from_str(&header) which panics on control
    // characters, DEL (0x7F), or non-ASCII bytes.
    if !header
        .bytes()
        .all(|b| b == b'\t' || (0x20u8..=0x7E).contains(&b))
    {
        bail!(
            "IDENTITY_HEADER contains characters that are not valid in HTTP headers \
             (control characters, DEL, or non-ASCII). \
             This value is set by the App Service platform and should not contain such characters."
        );
    }

    let suffix = vault_suffix()?;
    let resource = format!("https://{suffix}");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        // Never follow redirects — could leak the X-IDENTITY-HEADER.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client for App Service identity")?;

    let mut url =
        reqwest::Url::parse(&endpoint).context("failed to parse IDENTITY_ENDPOINT as URL")?;
    // Validate scheme — IDENTITY_ENDPOINT must be http or https to prevent
    // requests to file://, ftp://, or other unexpected protocols.
    if !["http", "https"].contains(&url.scheme()) {
        bail!(
            "IDENTITY_ENDPOINT must use http:// or https:// scheme, got: {}",
            url.scheme()
        );
    }
    url.query_pairs_mut()
        .append_pair("api-version", "2019-08-01")
        .append_pair("resource", &resource);
    if let Some(id) = client_id {
        url.query_pairs_mut().append_pair("client_id", id);
    }

    let request = client
        .get(url)
        .header("X-IDENTITY-HEADER", &header)
        .build()
        .context("failed to build App Service identity request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("App Service identity request failed")?;

    let status = response.status();
    if !status.is_success() {
        bail!(
            "App Service identity endpoint returned HTTP {}",
            status.as_u16()
        );
    }

    let token_resp: ImdsTokenResponse =
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(response.json()))
            .context("failed to parse App Service identity response")?;

    let expires_in = token_resp
        .expires_in
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok());

    Ok(TokenWithExpiry {
        token: token_resp.access_token,
        expires_in_secs: expires_in,
    })
}

/// Obtain a token via Azure Instance Metadata Service (IMDS) for Managed Identity.
///
/// Uses a short 2-second timeout since this will fail fast on non-Azure machines.
/// The IMDS endpoint is link-local (169.254.169.254) and only supports plain HTTP.
fn token_via_imds(client_id: Option<&str>) -> Result<TokenWithExpiry> {
    let suffix = vault_suffix()?;
    let resource = format!("https://{suffix}");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        // Never follow redirects on the link-local IMDS endpoint.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client for Azure IMDS")?;

    // Build URL with proper encoding via reqwest::Url to avoid injection.
    let mut url = reqwest::Url::parse("http://169.254.169.254/metadata/identity/oauth2/token")
        .context("failed to parse IMDS base URL")?;
    url.query_pairs_mut()
        .append_pair("api-version", "2018-02-01")
        .append_pair("resource", &resource);
    if let Some(id) = client_id {
        url.query_pairs_mut().append_pair("client_id", id);
    }

    let request = client
        .get(url)
        .header("Metadata", "true")
        .build()
        .context("failed to build Azure IMDS request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("Azure IMDS request failed")?;

    let status = response.status();
    if !status.is_success() {
        bail!("Azure IMDS returned HTTP {}", status.as_u16());
    }

    let token_resp: ImdsTokenResponse =
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(response.json()))
            .context("failed to parse Azure IMDS token response")?;

    let expires_in = token_resp
        .expires_in
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok());

    Ok(TokenWithExpiry {
        token: token_resp.access_token,
        expires_in_secs: expires_in,
    })
}

/// Obtain a token via the Azure CLI (`az account get-access-token`).
///
/// This supports developer workstations with an active `az login` session.
/// Uses `--scope` (v2.0 endpoint) for forward compatibility over the
/// deprecated `--resource` flag.
fn token_via_az_cli() -> Result<TokenWithExpiry> {
    let suffix = vault_suffix()?;
    let scope = format!("https://{suffix}/.default");

    let output = std::process::Command::new("az")
        .args([
            "account",
            "get-access-token",
            "--scope",
            &scope,
            "--output",
            "json",
        ])
        .stdin(std::process::Stdio::null())
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout =
                String::from_utf8(result.stdout).context("Azure CLI returned non-UTF-8 output")?;
            let parsed: serde_json::Value =
                serde_json::from_str(&stdout).context("failed to parse Azure CLI JSON output")?;
            let token = parsed["accessToken"]
                .as_str()
                .ok_or_else(|| anyhow!("Azure CLI output missing 'accessToken' field"))?;

            // `expiresOn` from Azure CLI is in the local system timezone,
            // not UTC. Treating it as UTC with `.and_utc()` produces
            // incorrect remaining lifetimes on non-UTC machines (e.g., a
            // developer in UTC+8 would see a token appear valid 8 hours
            // past its actual expiry, causing stale-token 401 errors).
            // Use None to fall back to the safe 50-minute default TTL.
            let expires_in_secs: Option<u64> = None;

            Ok(TokenWithExpiry {
                token: token.to_string(),
                expires_in_secs,
            })
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let stderr_snippet = truncate_body(stderr.trim(), ERROR_BODY_MAX_LEN);
            bail!(
                "Azure CLI `az account get-access-token` failed (exit {}): {}",
                result.status.code().unwrap_or(-1),
                stderr_snippet
            );
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            bail!("Azure CLI `az` not found on PATH");
        }
        Err(err) => {
            bail!("failed to execute Azure CLI: {err}");
        }
    }
}

/// Token response from the IMDS endpoint (uses `access_token` field).
#[derive(Deserialize)]
struct ImdsTokenResponse {
    access_token: String,
    /// Token lifetime in seconds.
    expires_in: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises tests that mutate shared environment variables.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn valid_reference_returns_vault_and_secret_names() {
        let r = AzureReference::parse("az://my-vault/my-secret").unwrap();
        assert_eq!(r.vault_name, "my-vault");
        assert_eq!(r.secret_name, "my-secret");
    }

    #[test]
    fn empty_reference_returns_error() {
        AzureReference::parse("az://").unwrap_err();
    }

    #[test]
    fn vault_without_secret_returns_error() {
        AzureReference::parse("az://my-vault").unwrap_err();
    }

    #[test]
    fn too_many_path_segments_returns_error() {
        AzureReference::parse("az://vault/secret/extra").unwrap_err();
    }

    #[test]
    fn wrong_scheme_returns_error() {
        AzureReference::parse("aws://vault/secret").unwrap_err();
    }

    #[test]
    fn dot_in_vault_name_returns_error() {
        AzureReference::parse("az://my.vault/secret").unwrap_err();
    }

    #[test]
    fn short_vault_name_returns_error() {
        AzureReference::parse("az://ab/secret").unwrap_err();
    }

    #[test]
    fn long_vault_name_returns_error() {
        let long_name = "a".repeat(25);
        let reference = format!("az://{long_name}/secret");
        AzureReference::parse(&reference).unwrap_err();
    }

    #[test]
    fn leading_hyphen_in_vault_name_returns_error() {
        AzureReference::parse("az://-vault/secret").unwrap_err();
    }

    #[test]
    fn consecutive_hyphens_in_vault_name_returns_error() {
        AzureReference::parse("az://my--vault/secret").unwrap_err();
    }

    #[test]
    fn hash_in_secret_name_returns_error() {
        AzureReference::parse("az://my-vault/sec#ret").unwrap_err();
    }

    #[test]
    fn whitespace_in_secret_name_returns_error() {
        AzureReference::parse("az://my-vault/my secret").unwrap_err();
    }

    #[test]
    fn sovereign_cloud_suffix_env_var_is_used() {
        // SAFETY: test-only, serialised with ENV_LOCK.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::set_var("AZURE_VAULT_SUFFIX", "vault.azure.cn") };
        let suffix = vault_suffix().unwrap();
        assert_eq!(suffix, "vault.azure.cn");
        unsafe { std::env::remove_var("AZURE_VAULT_SUFFIX") };
    }

    #[test]
    fn missing_suffix_env_var_uses_default() {
        // SAFETY: test-only, serialised with ENV_LOCK.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::remove_var("AZURE_VAULT_SUFFIX") };
        let suffix = vault_suffix().unwrap();
        assert_eq!(suffix, "vault.azure.net");
    }

    #[test]
    fn dangerous_chars_in_suffix_env_var_returns_error() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::set_var("AZURE_VAULT_SUFFIX", "evil.com/path#inject") };
        vault_suffix().unwrap_err();
        unsafe { std::env::remove_var("AZURE_VAULT_SUFFIX") };
    }

    #[test]
    fn valid_uuid_is_accepted() {
        validate_azure_id("12345678-abcd-ef01-2345-678901234567", "tenant").unwrap();
    }

    #[test]
    fn path_traversal_in_azure_id_returns_error() {
        validate_azure_id("../../../etc/passwd", "tenant").unwrap_err();
    }

}
