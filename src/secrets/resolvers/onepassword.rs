use anyhow::{anyhow, bail, Context, Result};
use secrecy::SecretString;

use crate::secrets::resolver::SecretResolver;
use crate::secrets::resolvers::validate_path_segment;

/// A parsed `op://vault/item/field` reference.
#[derive(Debug)]
struct OpReference {
    vault: String,
    item: String,
    field: String,
}

impl OpReference {
    fn parse(reference: &str) -> Result<Self> {
        let path = reference
            .strip_prefix("op://")
            .ok_or_else(|| anyhow!("invalid 1Password reference: must start with op://"))?;

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if segments.len() != 3 {
            bail!(
                "invalid 1Password reference: expected op://vault/item/field, got: {}",
                reference
            );
        }

        let vault = segments[0].to_string();
        let item = segments[1].to_string();
        let field = segments[2].to_string();

        validate_path_segment(&vault, "vault name")?;
        validate_path_segment(&item, "item name")?;
        validate_path_segment(&field, "field name")?;

        Ok(Self { vault, item, field })
    }
}

/// Resolver for 1Password secrets using the `op://` URI scheme.
///
/// Authentication uses the 1Password Connect Server API. Set both
/// `OP_CONNECT_TOKEN` and `OP_CONNECT_HOST` environment variables.
///
/// See <https://developer.1password.com/docs/connect/> for setup instructions.
///
/// References must be in the format `op://vault/item/field`, where `vault`
/// and `item` are human-readable names (or UUIDs). The resolver performs
/// name-to-UUID lookups via the Connect Server API.
pub struct OpResolver;

impl OpResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Authentication configuration resolved from environment variables.
struct OpAuth {
    host: String,
    token: String,
}

impl OpAuth {
    fn from_env() -> Result<Self> {
        let token = std::env::var("OP_CONNECT_TOKEN").ok().filter(|t| !t.is_empty());
        let host = std::env::var("OP_CONNECT_HOST").ok().filter(|h| !h.is_empty());

        match (token, host) {
            (Some(token), Some(host)) => Ok(Self { host, token }),
            _ => bail!(
                "1Password secret resolution requires OP_CONNECT_TOKEN + OP_CONNECT_HOST \
                 environment variables. Earl uses the 1Password Connect Server API. \
                 See https://developer.1password.com/docs/connect/"
            ),
        }
    }
}

impl SecretResolver for OpResolver {
    fn scheme(&self) -> &str {
        "op"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let op_ref = OpReference::parse(reference)?;
        let auth = OpAuth::from_env()?;

        let base = auth.host.trim_end_matches('/');

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client for 1Password")?;

        // Step 1: Resolve vault name to UUID.
        let vault_id =
            resolve_vault_id(&client, base, &auth.token, &op_ref.vault)?;

        // Step 2: Resolve item name to UUID within the vault.
        let item_id =
            resolve_item_id(&client, base, &auth.token, &vault_id, &op_ref.item)?;

        // Step 3: Fetch the full item (with fields) by UUID.
        let item_url = format!("{base}/v1/vaults/{vault_id}/items/{item_id}");

        let request = client
            .get(&item_url)
            .header("Authorization", format!("Bearer {}", auth.token))
            .header("Accept", "application/json")
            .build()
            .context("failed to build 1Password item request")?;

        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.execute(request))
        })
        .context("1Password API request failed")?;

        let status = response.status();
        if !status.is_success() {
            let body = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(response.text())
            })
            .unwrap_or_default();
            bail!(
                "1Password API returned HTTP {}: {}",
                status.as_u16(),
                body
            );
        }

        let body: serde_json::Value = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.json())
        })
        .context("failed to parse 1Password API response")?;

        // The response has a `fields` array; find the one matching our field label or id.
        let fields = body["fields"]
            .as_array()
            .ok_or_else(|| anyhow!("1Password API response missing 'fields' array"))?;

        let field_value = fields
            .iter()
            .find(|f| {
                f["label"].as_str() == Some(&op_ref.field)
                    || f["id"].as_str() == Some(&op_ref.field)
            })
            .and_then(|f| f["value"].as_str())
            .ok_or_else(|| {
                anyhow!(
                    "field '{}' not found in 1Password item '{}/{}' (available fields: {})",
                    op_ref.field,
                    op_ref.vault,
                    op_ref.item,
                    fields
                        .iter()
                        .filter_map(|f| f["label"].as_str().or(f["id"].as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        Ok(SecretString::from(field_value.to_string()))
    }
}

/// Resolve a vault name to its UUID via the Connect Server API.
///
/// If `name_or_id` looks like a UUID (contains only hex digits and hyphens with
/// the right length), it is returned as-is.
fn resolve_vault_id(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    name_or_id: &str,
) -> Result<String> {
    if looks_like_uuid(name_or_id) {
        return Ok(name_or_id.to_string());
    }

    let url = format!("{base}/v1/vaults?filter=name eq \"{name_or_id}\"");
    let request = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .build()
        .context("failed to build 1Password vault lookup request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("1Password vault lookup request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.text())
        })
        .unwrap_or_default();
        bail!(
            "1Password vault lookup returned HTTP {}: {}",
            status.as_u16(),
            body
        );
    }

    let vaults: Vec<serde_json::Value> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse 1Password vault lookup response")?;

    let vault = vaults.first().ok_or_else(|| {
        anyhow!("1Password vault '{}' not found", name_or_id)
    })?;

    vault["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("1Password vault response missing 'id' field"))
}

/// Resolve an item title to its UUID within a vault.
///
/// If `name_or_id` looks like a UUID, it is returned as-is.
fn resolve_item_id(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    vault_id: &str,
    name_or_id: &str,
) -> Result<String> {
    if looks_like_uuid(name_or_id) {
        return Ok(name_or_id.to_string());
    }

    let url = format!(
        "{base}/v1/vaults/{vault_id}/items?filter=title eq \"{name_or_id}\""
    );
    let request = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        .build()
        .context("failed to build 1Password item lookup request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("1Password item lookup request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.text())
        })
        .unwrap_or_default();
        bail!(
            "1Password item lookup returned HTTP {}: {}",
            status.as_u16(),
            body
        );
    }

    let items: Vec<serde_json::Value> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse 1Password item lookup response")?;

    let item = items.first().ok_or_else(|| {
        anyhow!("1Password item '{}' not found in vault", name_or_id)
    })?;

    item["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("1Password item response missing 'id' field"))
}

/// Quick heuristic to detect UUID-formatted strings (e.g. `26ydsakjlkxyz...`).
/// 1Password UUIDs are 26-character alphanumeric strings.
fn looks_like_uuid(s: &str) -> bool {
    s.len() == 26 && s.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_reference() {
        let r = OpReference::parse("op://my-vault/my-item/password").unwrap();
        assert_eq!(r.vault, "my-vault");
        assert_eq!(r.item, "my-item");
        assert_eq!(r.field, "password");
    }

    #[test]
    fn parse_rejects_too_few_segments() {
        let err = OpReference::parse("op://vault/item").unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn parse_rejects_empty_path() {
        let err = OpReference::parse("op://").unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let err = OpReference::parse("vault://a/b/c").unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn parse_rejects_control_char_in_vault() {
        let err = OpReference::parse("op://my\x00vault/item/field").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_question_mark_in_item() {
        let err = OpReference::parse("op://vault/item?q=1/field").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_hash_in_field() {
        let err = OpReference::parse("op://vault/item/field#frag").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_whitespace_in_vault() {
        let err = OpReference::parse("op://my vault/item/field").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }
}
