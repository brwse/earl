use std::collections::HashMap;

use anyhow::{Context, Result, anyhow, bail};
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
        let (full_path, field) = after_scheme.split_once('#').ok_or_else(|| {
            anyhow!("invalid Vault reference: missing '#field' suffix in {reference}")
        })?;

        if field.is_empty() {
            bail!("invalid Vault reference: field after '#' must not be empty in {reference}");
        }

        // The full_path is mount/path where the first segment is the mount point
        // and the rest is the secret path within that mount.
        let segments: Vec<&str> = full_path.split('/').collect();

        // Reject empty segments from double slashes or trailing slashes —
        // e.g., `vault://secret//path#field` could silently misresolve.
        if segments.iter().any(|s| s.is_empty()) {
            bail!(
                "invalid Vault reference: contains empty path segments \
                 (double slash or trailing slash) in {reference}"
            );
        }

        if segments.len() < 2 {
            bail!("invalid Vault reference: expected vault://mount/path#field, got: {reference}");
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

/// Returns `true` when every byte in `s` is valid in an HTTP header value
/// (printable ASCII 0x20–0x7E, or tab 0x09).
fn is_header_safe(s: &str) -> bool {
    s.bytes()
        .all(|b| b == b'\t' || (0x20u8..=0x7E).contains(&b))
}

/// Resolver for HashiCorp Vault secrets using the `vault://` URI scheme.
///
/// Reads secrets from a Vault KV v2 secrets engine. Requires the following
/// environment variables:
///
/// * `VAULT_ADDR` — the Vault server address (e.g. `https://vault.example.com:8200`)
/// * `VAULT_TOKEN` — a valid Vault authentication token
///
/// Optional environment variables:
///
/// * `VAULT_NAMESPACE` — Vault enterprise namespace (e.g. `admin/team-a`)
/// * `VAULT_SKIP_VERIFY` — set to `"1"` or `"true"` to disable TLS verification
///
/// TLS is verified against the system certificate store by default.
/// `VAULT_CACERT` (path to a PEM CA certificate file) and `VAULT_CAPATH`
/// (path to a directory of PEM CA certificates) are read automatically by
/// the underlying `vaultrs` library and can be used to trust a private CA.
/// `VAULT_CLIENT_CERT` and `VAULT_CLIENT_KEY` are also read by `vaultrs`
/// for mTLS client authentication.
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

        // Warn if the secret path starts with "data/" — this is almost always
        // a mistake because `vaultrs::kv2::read` automatically prepends `data/`
        // to the path (KV v2 convention). A path like `data/myapp` would become
        // `secret/data/data/myapp` at the API level.
        if vault_ref.path.starts_with("data/") || vault_ref.path == "data" {
            bail!(
                "Vault secret path '{}' starts with 'data/' — this is likely a mistake. \
                 Earl uses the KV v2 API which automatically adds the 'data/' prefix. \
                 Use the path without 'data/' (e.g., 'myapp' instead of 'data/myapp').",
                vault_ref.path
            );
        }

        let addr = std::env::var("VAULT_ADDR")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "VAULT_ADDR is not set. Set both VAULT_ADDR and VAULT_TOKEN \
                     environment variables to use vault:// secret references. \
                     For enterprise Vault with namespaces, also set VAULT_NAMESPACE."
                )
            })?;

        let token = std::env::var("VAULT_TOKEN")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "VAULT_TOKEN is not set. Set both VAULT_ADDR and VAULT_TOKEN \
                     environment variables to use vault:// secret references. \
                     For enterprise Vault with namespaces, also set VAULT_NAMESPACE."
                )
            })?;

        // Validate VAULT_TOKEN contains only HTTP header-safe bytes.
        // vaultrs calls HeaderValue::from_str(&token).unwrap() which panics on
        // control characters, DEL (0x7F), or non-ASCII bytes.
        if !is_header_safe(&token) {
            bail!(
                "VAULT_TOKEN contains characters that are not valid in HTTP headers \
                 (control characters, DEL, or non-ASCII). \
                 Vault tokens must consist only of printable ASCII characters."
            );
        }

        let namespace = std::env::var("VAULT_NAMESPACE")
            .ok()
            .filter(|v| !v.is_empty());

        // Pre-validate the URL to produce a clear error instead of the panic
        // inside vaultrs's address() setter which calls unwrap() on url::Url::parse.
        let parsed_addr = reqwest::Url::parse(&addr).with_context(|| {
            format!(
                "VAULT_ADDR is not a valid URL: {addr}. \
                 Expected format: https://vault.example.com:8200"
            )
        })?;
        if !["http", "https"].contains(&parsed_addr.scheme()) {
            bail!(
                "VAULT_ADDR must use http:// or https:// scheme, got: {}",
                parsed_addr.scheme()
            );
        }

        let mut settings_builder = VaultClientSettingsBuilder::default();
        settings_builder.address(&addr).token(token);

        if let Some(ref ns) = namespace {
            // Validate namespace contains only HTTP header-safe bytes.
            // vaultrs calls HeaderValue::from_str(ns).unwrap() which panics on
            // control characters, DEL (0x7F), or non-ASCII bytes.
            if !is_header_safe(ns) {
                bail!(
                    "VAULT_NAMESPACE contains characters that are not valid in HTTP headers \
                     (control characters, DEL, or non-ASCII). \
                     Must be a valid Vault namespace path (e.g., 'admin/team-a')."
                );
            }
            settings_builder.set_namespace(ns.clone());
        }

        // Always set verify explicitly — vaultrs 0.7.x has an inverted
        // default_verify() implementation that maps VAULT_SKIP_VERIFY=true to
        // verify=true (the opposite of the intended behavior). By always calling
        // settings_builder.verify(), we bypass the buggy default and ensure
        // correct behavior regardless of whether VAULT_SKIP_VERIFY is set.
        let skip_verify = std::env::var("VAULT_SKIP_VERIFY")
            .ok()
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "t"))
            .unwrap_or(false);
        settings_builder.verify(!skip_verify);

        // VAULT_CACERT, VAULT_CAPATH, VAULT_CLIENT_CERT, and VAULT_CLIENT_KEY
        // are read automatically by vaultrs's builder defaults (default_ca_certs
        // and default_identity). No explicit configuration is needed here.
        // Use VAULT_SKIP_VERIFY=1 only as a last resort (not in production).

        let settings = settings_builder
            .build()
            .context("failed to build Vault client settings")?;

        let client = VaultClient::new(settings).context("failed to create Vault client")?;

        // We are inside a sync trait method but need to perform an async API call.
        // Use tokio's block_in_place + Handle::current().block_on() to bridge.
        let secret_data: HashMap<String, serde_json::Value> = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(vaultrs::kv2::read(
                &client,
                &vault_ref.mount,
                &vault_ref.path,
            ))
        })
        .with_context(|| {
            let ns_hint = namespace
                .as_deref()
                .map(|ns| format!(" (namespace='{ns}')"))
                .unwrap_or_default();
            format!(
                "failed to read Vault secret at mount='{}', path='{}'{ns_hint}. \
                 Note: Earl uses KV v2 — ensure the mount uses the KV v2 secrets engine.",
                vault_ref.mount, vault_ref.path
            )
        })?;

        let value = secret_data.get(&vault_ref.field).ok_or_else(|| {
            let ns_hint = namespace
                .as_deref()
                .map(|ns| format!(" (namespace='{ns}')"))
                .unwrap_or_default();
            anyhow!(
                "field '{}' not found in Vault secret '{}/{}'{ns_hint}. \
                 Verify the field name matches a top-level key in the secret's data map.",
                vault_ref.field,
                vault_ref.mount,
                vault_ref.path,
            )
        })?;

        // Extract string value — if it's a JSON string, unwrap it;
        // otherwise serialize it back to a JSON string.
        let text = match value.as_str() {
            Some(s) => s.to_string(),
            None => value.to_string(),
        };

        Ok(SecretString::from(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_reference_mount_is_first_path_segment() {
        let r = VaultReference::parse("vault://secret/myapp#api_key").unwrap();
        assert_eq!(r.mount, "secret");
    }

    #[test]
    fn valid_reference_path_excludes_mount_segment() {
        let r = VaultReference::parse("vault://secret/myapp#api_key").unwrap();
        assert_eq!(r.path, "myapp");
    }

    #[test]
    fn valid_reference_field_is_fragment_portion() {
        let r = VaultReference::parse("vault://secret/myapp#api_key").unwrap();
        assert_eq!(r.field, "api_key");
    }

    #[test]
    fn nested_path_mount_is_first_segment() {
        let r = VaultReference::parse("vault://secret/data/team/app#password").unwrap();
        assert_eq!(r.mount, "secret");
    }

    #[test]
    fn nested_path_joins_all_segments_after_mount() {
        let r = VaultReference::parse("vault://secret/data/team/app#password").unwrap();
        assert_eq!(r.path, "data/team/app");
    }

    #[test]
    fn nested_path_field_is_preserved() {
        let r = VaultReference::parse("vault://secret/data/team/app#password").unwrap();
        assert_eq!(r.field, "password");
    }

    #[test]
    fn reference_without_hash_field_returns_error() {
        VaultReference::parse("vault://secret/myapp").unwrap_err();
    }

    #[test]
    fn empty_field_name_after_hash_returns_error() {
        VaultReference::parse("vault://secret/myapp#").unwrap_err();
    }

    #[test]
    fn missing_path_after_scheme_returns_error() {
        VaultReference::parse("vault://#field").unwrap_err();
    }

    #[test]
    fn mount_without_path_segment_returns_error() {
        VaultReference::parse("vault://secret#field").unwrap_err();
    }

    #[test]
    fn non_vault_scheme_returns_error() {
        VaultReference::parse("op://vault/item/field").unwrap_err();
    }

    #[test]
    fn empty_uri_body_returns_error() {
        VaultReference::parse("vault://").unwrap_err();
    }

    #[test]
    fn question_mark_in_mount_returns_error() {
        VaultReference::parse("vault://sec?ret/path#field").unwrap_err();
    }

    #[test]
    fn whitespace_in_path_segment_returns_error() {
        VaultReference::parse("vault://secret/my path#field").unwrap_err();
    }

    #[test]
    fn control_char_in_field_name_returns_error() {
        VaultReference::parse("vault://secret/path#fi\x00eld").unwrap_err();
    }

    #[test]
    fn data_prefix_in_path_does_not_cause_parse_error() {
        // Parsing itself should succeed — the data/ prefix warning is in resolve().
        VaultReference::parse("vault://secret/data/myapp#field").unwrap();
    }

    #[test]
    fn data_prefix_in_path_is_preserved_in_parsed_path() {
        let r = VaultReference::parse("vault://secret/data/myapp#field").unwrap();
        assert_eq!(r.path, "data/myapp");
    }

    #[test]
    fn token_rejects_newline() {
        assert!(!is_header_safe("tok\nen"));
    }

    #[test]
    fn token_rejects_del() {
        assert!(!is_header_safe("tok\x7Fen"));
    }

    #[test]
    fn token_rejects_non_ascii() {
        assert!(!is_header_safe("tök"));
    }

    #[test]
    fn legacy_format_vault_token_passes_header_safety_check() {
        // Vault tokens look like: s.XhzOVFgiTw3n3OYJqBiqIGfx
        assert!(is_header_safe("s.XhzOVFgiTw3n3OYJqBiqIGfx"));
    }

    #[test]
    fn hvs_format_vault_token_passes_header_safety_check() {
        // Vault tokens look like: hvs.XXXX
        assert!(is_header_safe("hvs.CAESIBtR0QkDnWL0oFKj9iC8AAAA"));
    }

    #[test]
    fn namespace_rejects_del() {
        assert!(!is_header_safe("admin\x7F/team"));
    }

    #[test]
    fn namespace_rejects_non_ascii() {
        assert!(!is_header_safe("admin/tëam"));
    }

    #[test]
    fn namespace_with_path_separator_passes_header_safety_check() {
        assert!(is_header_safe("admin/team-a"));
    }

    #[test]
    fn simple_namespace_passes_header_safety_check() {
        assert!(is_header_safe("root"));
    }
}
