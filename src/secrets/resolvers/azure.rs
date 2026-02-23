use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use secrecy::SecretString;
use serde::Deserialize;

use crate::secrets::resolver::SecretResolver;
use crate::secrets::resolvers::{validate_azure_vault_name, validate_path_segment};

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
            bail!("invalid Azure reference: vault name and secret name are required in {reference}");
        }

        let segments: Vec<&str> = after_scheme.split('/').filter(|s| !s.is_empty()).collect();

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
/// Authentication uses Azure AD client credentials (service principal) via the
/// following environment variables:
///
/// * `AZURE_TENANT_ID` — the Azure AD tenant (directory) ID
/// * `AZURE_CLIENT_ID` — the application (client) ID
/// * `AZURE_CLIENT_SECRET` — the client secret
///
/// References use the format `az://vault-name/secret-name`, where:
/// * `vault-name` is the Azure Key Vault name (used to form `https://{vault-name}.vault.azure.net`)
/// * `secret-name` is the secret identifier within the vault
///
/// Example: `az://my-vault/api-key`
pub struct AzureResolver;

impl AzureResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AzureResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretResolver for AzureResolver {
    fn scheme(&self) -> &str {
        "az"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let az_ref = AzureReference::parse(reference)?;

        let access_token =
            obtain_access_token(&az_ref.vault_name).context("failed to obtain Azure AD access token")?;

        let url = format!(
            "https://{}.vault.azure.net/secrets/{}?api-version=7.4",
            az_ref.vault_name, az_ref.secret_name
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client for Azure Key Vault")?;

        let request = client
            .get(&url)
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
                body
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
}

/// Obtain an Azure AD access token using client credentials flow.
///
/// Requires `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, and `AZURE_CLIENT_SECRET`
/// environment variables to be set.
fn obtain_access_token(vault_name: &str) -> Result<String> {
    let tenant_id = std::env::var("AZURE_TENANT_ID")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("AZURE_TENANT_ID environment variable is not set"))?;

    let client_id = std::env::var("AZURE_CLIENT_ID")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("AZURE_CLIENT_ID environment variable is not set"))?;

    let client_secret = std::env::var("AZURE_CLIENT_SECRET")
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            anyhow!("AZURE_CLIENT_SECRET environment variable is not set")
        })?;

    let token_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        tenant_id
    );

    // The scope for Azure Key Vault is https://{vault-name}.vault.azure.net/.default
    // but the standard scope works across all vaults.
    let scope = format!("https://{}.vault.azure.net/.default", vault_name);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client for Azure AD token exchange")?;

    let request = client
        .post(&token_url)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("scope", &scope),
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
            body
        );
    }

    let token_resp: TokenResponse = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse Azure AD token response")?;

    Ok(token_resp.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vault_and_secret() {
        let r = AzureReference::parse("az://my-vault/my-secret").unwrap();
        assert_eq!(r.vault_name, "my-vault");
        assert_eq!(r.secret_name, "my-secret");
    }

    #[test]
    fn parse_rejects_empty() {
        let err = AzureReference::parse("az://").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_vault_only() {
        let err = AzureReference::parse("az://my-vault").unwrap_err();
        assert!(
            err.to_string().contains("invalid") || err.to_string().contains("expected"),
            "got: {}",
            err
        );
    }

    #[test]
    fn parse_rejects_too_many_segments() {
        let err = AzureReference::parse("az://vault/secret/extra").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let err = AzureReference::parse("aws://vault/secret").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_dot_in_vault_name() {
        let err = AzureReference::parse("az://my.vault/secret").unwrap_err();
        assert!(
            err.to_string().contains("alphanumeric"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_short_vault_name() {
        let err = AzureReference::parse("az://ab/secret").unwrap_err();
        assert!(err.to_string().contains("3-24"), "got: {err}");
    }

    #[test]
    fn parse_rejects_long_vault_name() {
        let long_name = "a".repeat(25);
        let reference = format!("az://{long_name}/secret");
        let err = AzureReference::parse(&reference).unwrap_err();
        assert!(err.to_string().contains("3-24"), "got: {err}");
    }

    #[test]
    fn parse_rejects_leading_hyphen_vault() {
        let err = AzureReference::parse("az://-vault/secret").unwrap_err();
        assert!(
            err.to_string().contains("must not start or end"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_consecutive_hyphens_in_vault() {
        let err = AzureReference::parse("az://my--vault/secret").unwrap_err();
        assert!(
            err.to_string().contains("consecutive hyphens"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_hash_in_secret_name() {
        let err = AzureReference::parse("az://my-vault/sec#ret").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_whitespace_in_secret_name() {
        let err = AzureReference::parse("az://my-vault/my secret").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }
}
