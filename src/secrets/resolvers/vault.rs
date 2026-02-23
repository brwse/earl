use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use secrecy::SecretString;
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

use crate::secrets::resolver::SecretResolver;
use crate::secrets::resolvers::validate_path_segment;

/// A parsed `vault://mount/path#field` reference.
#[derive(Debug)]
struct VaultReference {
    mount: String,
    path: String,
    field: String,
}

impl VaultReference {
    fn parse(reference: &str) -> Result<Self> {
        let after_scheme = reference
            .strip_prefix("vault://")
            .ok_or_else(|| anyhow!("invalid Vault reference: must start with vault://"))?;

        // Split on '#' to separate path from field
        let (full_path, field) = after_scheme
            .split_once('#')
            .ok_or_else(|| anyhow!("invalid Vault reference: missing '#field' suffix in {reference}"))?;

        if field.is_empty() {
            bail!("invalid Vault reference: field after '#' must not be empty in {reference}");
        }

        // The full_path is mount/path where the first segment is the mount point
        // and the rest is the secret path within that mount.
        let segments: Vec<&str> = full_path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.len() < 2 {
            bail!(
                "invalid Vault reference: expected vault://mount/path#field, got: {reference}"
            );
        }

        let mount = segments[0].to_string();
        let path = segments[1..].join("/");

        validate_path_segment(&mount, "mount point")?;
        for segment in &segments[1..] {
            validate_path_segment(segment, "secret path segment")?;
        }
        validate_path_segment(field, "field name")?;

        Ok(Self {
            mount,
            path,
            field: field.to_string(),
        })
    }
}

/// Resolver for HashiCorp Vault secrets using the `vault://` URI scheme.
///
/// Reads secrets from a Vault KV v2 secrets engine. Requires the following
/// environment variables:
///
/// * `VAULT_ADDR` — the Vault server address (e.g. `https://vault.example.com:8200`)
/// * `VAULT_TOKEN` — a valid Vault authentication token
///
/// References use the format `vault://mount/path#field`, where:
/// * `mount` is the secrets engine mount point (commonly `"secret"`)
/// * `path` is the secret path within the mount
/// * `field` is the key to extract from the secret's data map
///
/// Example: `vault://secret/myapp#api_key`
pub struct VaultResolver;

impl VaultResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VaultResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretResolver for VaultResolver {
    fn scheme(&self) -> &str {
        "vault"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let vault_ref = VaultReference::parse(reference)?;

        let addr = std::env::var("VAULT_ADDR")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "Vault credentials not found. Set both VAULT_ADDR and VAULT_TOKEN \
                     environment variables to use vault:// secret references."
                )
            })?;

        let token = std::env::var("VAULT_TOKEN")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "Vault credentials not found. Set both VAULT_ADDR and VAULT_TOKEN \
                     environment variables to use vault:// secret references."
                )
            })?;

        let settings = VaultClientSettingsBuilder::default()
            .address(&addr)
            .token(token)
            .build()
            .context("failed to build Vault client settings")?;

        let client = VaultClient::new(settings)
            .context("failed to create Vault client")?;

        // We are inside a sync trait method but need to perform an async API call.
        // Use tokio's block_in_place + Handle::current().block_on() to bridge.
        let secret_data: HashMap<String, String> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(vaultrs::kv2::read(&client, &vault_ref.mount, &vault_ref.path))
        })
        .with_context(|| {
            format!(
                "failed to read Vault secret at mount='{}', path='{}'",
                vault_ref.mount, vault_ref.path
            )
        })?;

        let value = secret_data.get(&vault_ref.field).ok_or_else(|| {
            anyhow!(
                "field '{}' not found in Vault secret '{}/{}' (available fields: {})",
                vault_ref.field,
                vault_ref.mount,
                vault_ref.path,
                secret_data
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

        Ok(SecretString::from(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_reference() {
        let r = VaultReference::parse("vault://secret/myapp#api_key").unwrap();
        assert_eq!(r.mount, "secret");
        assert_eq!(r.path, "myapp");
        assert_eq!(r.field, "api_key");
    }

    #[test]
    fn parse_nested_path() {
        let r = VaultReference::parse("vault://secret/data/team/app#password").unwrap();
        assert_eq!(r.mount, "secret");
        assert_eq!(r.path, "data/team/app");
        assert_eq!(r.field, "password");
    }

    #[test]
    fn parse_rejects_missing_field() {
        let err = VaultReference::parse("vault://secret/myapp").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_empty_field() {
        let err = VaultReference::parse("vault://secret/myapp#").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_empty_path() {
        let err = VaultReference::parse("vault://#field").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_mount_only() {
        let err = VaultReference::parse("vault://secret#field").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let err = VaultReference::parse("op://vault/item/field").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_empty_uri() {
        let err = VaultReference::parse("vault://").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_question_mark_in_mount() {
        let err = VaultReference::parse("vault://sec?ret/path#field").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_whitespace_in_path() {
        let err = VaultReference::parse("vault://secret/my path#field").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_control_char_in_field() {
        let err = VaultReference::parse("vault://secret/path#fi\x00eld").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }
}
