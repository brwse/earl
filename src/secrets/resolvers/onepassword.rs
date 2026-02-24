use anyhow::{anyhow, bail, Context, Result};
use secrecy::{ExposeSecret, SecretString};

use crate::secrets::resolver::SecretResolver;
use crate::secrets::resolvers::{truncate_body, ERROR_BODY_MAX_LEN};

/// Validate a 1Password reference segment (vault, item, section, or field name).
///
/// Unlike the shared `validate_path_segment`, this allows spaces because
/// 1Password vault and item names commonly contain them (e.g., "Work Credentials").
/// Rejects only characters that are dangerous in URIs or SCIM filters.
fn validate_op_segment(value: &str, field_name: &str) -> Result<()> {
    if value.is_empty() {
        bail!("{field_name} must not be empty");
    }
    for ch in value.chars() {
        if ch == '/' || ch == '?' || ch == '#' || ch.is_control() {
            bail!(
                "{field_name} contains invalid character '{}' — \
                 must not contain '/', '?', '#', or control characters",
                ch.escape_debug()
            );
        }
    }
    Ok(())
}

/// A parsed `op://vault/item/field` or `op://vault/item/section/field` reference.
#[derive(Debug)]
struct OpReference {
    vault: String,
    item: String,
    /// The field path — for 4-segment references this is `section/field`.
    field: String,
}

impl OpReference {
    fn parse(reference: &str) -> Result<Self> {
        let path = reference
            .strip_prefix("op://")
            .ok_or_else(|| anyhow!("invalid 1Password reference: must start with op://"))?;

        let segments: Vec<&str> = path.split('/').collect();

        // Reject empty segments from double slashes or trailing slashes —
        // e.g., `op://vault//item/field` could silently misresolve.
        if segments.iter().any(|s| s.is_empty()) {
            bail!(
                "invalid 1Password reference: contains empty path segments \
                 (double slash or trailing slash) in {reference}"
            );
        }

        match segments.len() {
            3 => {
                let vault = segments[0].to_string();
                let item = segments[1].to_string();
                let field = segments[2].to_string();

                validate_op_segment(&vault, "vault name")?;
                validate_op_segment(&item, "item name")?;
                validate_op_segment(&field, "field name")?;

                Ok(Self { vault, item, field })
            }
            4 => {
                // op://vault/item/section/field — section is part of the field path.
                let vault = segments[0].to_string();
                let item = segments[1].to_string();
                let section = segments[2];
                let field = segments[3];

                validate_op_segment(&vault, "vault name")?;
                validate_op_segment(&item, "item name")?;
                validate_op_segment(section, "section name")?;
                validate_op_segment(field, "field name")?;

                Ok(Self {
                    vault,
                    item,
                    field: format!("{section}/{field}"),
                })
            }
            _ => {
                bail!(
                    "invalid 1Password reference: expected op://vault/item/field \
                     or op://vault/item/section/field, got: {}",
                    reference
                );
            }
        }
    }
}

/// Characters that are unsafe inside SCIM filter string literals.
///
/// The Connect Server API uses SCIM filters like `name eq "value"`.
/// We reject `"` and `\` to prevent filter injection.
const SCIM_UNSAFE_CHARS: &[char] = &['"', '\\'];

/// Resolver for 1Password secrets using the `op://` URI scheme.
///
/// Authentication is attempted in this order:
///
/// 1. **Connect Server API** — when both `OP_CONNECT_TOKEN` and `OP_CONNECT_HOST`
///    environment variables are set. See <https://developer.1password.com/docs/connect/>
/// 2. **`op` CLI fallback** — runs `op read <reference>` as a subprocess. This
///    supports `OP_SERVICE_ACCOUNT_TOKEN` (non-interactive CI/CD) and interactive
///    `op signin` sessions (developer workstations).
///
/// References must be in the format `op://vault/item/field`, where `vault`
/// and `item` are human-readable names (or UUIDs). The resolver performs
/// name-to-UUID lookups via the Connect Server API (path 1) or delegates
/// resolution to the `op` CLI (path 2).
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
    token: SecretString,
}

impl OpAuth {
    fn from_env() -> Result<Self> {
        let token = std::env::var("OP_CONNECT_TOKEN").ok().filter(|t| !t.is_empty());
        let host = std::env::var("OP_CONNECT_HOST").ok().filter(|h| !h.is_empty());

        match (token, host) {
            (Some(token), Some(host)) => {
                // Validate OP_CONNECT_HOST is a valid HTTP(S) URL.
                let parsed = reqwest::Url::parse(&host)
                    .with_context(|| format!("OP_CONNECT_HOST is not a valid URL: {host}"))?;
                match parsed.scheme() {
                    "https" => {}
                    "http" => {
                        tracing::warn!(
                            "OP_CONNECT_HOST uses plain HTTP — the bearer token will be \
                             transmitted in cleartext. Use HTTPS in production."
                        );
                    }
                    other => bail!(
                        "OP_CONNECT_HOST must use http:// or https:// scheme, got: {other}"
                    ),
                }
                if !parsed.path().is_empty() && parsed.path() != "/" {
                    bail!(
                        "OP_CONNECT_HOST must not include a path, got: {host}"
                    );
                }
                if !parsed.username().is_empty() || parsed.password().is_some() {
                    bail!(
                        "OP_CONNECT_HOST must not include userinfo (username/password), got: {host}"
                    );
                }
                if parsed.query().is_some() {
                    bail!(
                        "OP_CONNECT_HOST must not include a query string, got: {host}"
                    );
                }
                if parsed.fragment().is_some() {
                    bail!(
                        "OP_CONNECT_HOST must not include a fragment, got: {host}"
                    );
                }
                Ok(Self {
                    host: host.trim_end_matches('/').to_string(),
                    token: SecretString::from(token),
                })
            }
            _ => bail!(
                "1Password Connect Server requires OP_CONNECT_TOKEN + OP_CONNECT_HOST \
                 environment variables. See https://developer.1password.com/docs/connect/"
            ),
        }
    }
}

/// Returns true if `s` looks like a 1Password UUID (exactly 26 characters,
/// Crockford base32 alphabet: uppercase A–Z and digits 2–7).
///
/// 1Password UUIDs are generated in this specific alphabet and length, so
/// the false-positive rate on human-readable vault/item names is negligible.
fn is_op_uuid(s: &str) -> bool {
    s.len() == 26 && s.chars().all(|c| c.is_ascii_uppercase() || matches!(c, '2'..='7'))
}

/// Validate that a value is safe to use in a SCIM filter string literal.
fn validate_scim_value(value: &str, field_name: &str) -> Result<()> {
    for ch in value.chars() {
        if SCIM_UNSAFE_CHARS.contains(&ch) || ch.is_control() {
            bail!(
                "{field_name} contains invalid character '{}' — \
                 must not contain '\"', '\\', or control characters (SCIM filter injection risk)",
                ch.escape_debug()
            );
        }
    }
    Ok(())
}

impl SecretResolver for OpResolver {
    fn scheme(&self) -> &str {
        "op"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let op_ref = OpReference::parse(reference)?;

        // Reconstruct the canonical reference from the parsed struct so the
        // CLI receives exactly what the parser validated (avoids double-slash
        // or other normalization differences from the raw user input).
        let canonical_ref = format!("op://{}/{}/{}", op_ref.vault, op_ref.item, op_ref.field);

        // Try Connect Server first, fall back to `op` CLI.
        match OpAuth::from_env() {
            Ok(auth) => resolve_via_connect(&op_ref, &auth),
            Err(_) => resolve_via_cli(&canonical_ref),
        }
    }
}

/// Resolve a secret via the 1Password Connect Server API.
///
/// Both 3-segment (`op://vault/item/field`) and 4-segment
/// (`op://vault/item/section/field`) references are supported. Section-scoped
/// lookups are handled client-side: the item is fetched in full and the
/// `section.label`/`section.id` fields in the response are matched locally.
fn resolve_via_connect(op_ref: &OpReference, auth: &OpAuth) -> Result<SecretString> {
    // The Connect Server API uses SCIM filters; validate names are injection-safe.
    validate_scim_value(&op_ref.vault, "vault name")?;
    validate_scim_value(&op_ref.item, "item name")?;

    let base = auth.host.trim_end_matches('/');
    let token = auth.token.expose_secret();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        // Never follow redirects — a redirect carrying the Authorization header
        // could silently leak the bearer token to a third-party host.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("failed to build HTTP client for 1Password")?;

    // Step 1: Resolve vault name to UUID.
    let vault_id = resolve_vault_id(&client, base, token, &op_ref.vault)?;

    // Step 2: Resolve item name to UUID within the vault.
    let item_id = resolve_item_id(&client, base, token, &vault_id, &op_ref.item)?;

    // Step 3: Fetch the full item (with fields) by UUID.
    let item_url = format!("{base}/v1/vaults/{vault_id}/items/{item_id}");

    let request = client
        .get(&item_url)
        .header("Authorization", format!("Bearer {token}"))
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
            truncate_body(&body, ERROR_BODY_MAX_LEN)
        );
    }

    let body: serde_json::Value = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse 1Password API response")?;

    // The response has a `fields` array; find the one matching our field label or id.
    // For section-scoped fields (section/field), we match the field label directly
    // since the Connect API does not natively scope by section.
    let fields = body["fields"]
        .as_array()
        .ok_or_else(|| anyhow!("1Password API response missing 'fields' array"))?;

    // For section-scoped fields (e.g., "login/password"), match on both
    // the section label and the field label from the Connect API response.
    let (expected_section, expected_field) = if op_ref.field.contains('/') {
        let (s, f) = op_ref.field.split_once('/').unwrap();
        (Some(s), f)
    } else {
        (None, op_ref.field.as_str())
    };

    let field_value = fields
        .iter()
        .find(|f| {
            let label_matches = f["label"].as_str() == Some(expected_field)
                || f["id"].as_str() == Some(expected_field);
            if !label_matches {
                return false;
            }
            // If a section was specified, verify it matches.
            match expected_section {
                Some(section) => {
                    f["section"]["label"].as_str() == Some(section)
                        || f["section"]["id"].as_str() == Some(section)
                }
                None => true,
            }
        })
        .and_then(|f| f["value"].as_str())
        .ok_or_else(|| {
            anyhow!(
                "field '{}' not found in 1Password item '{}/{}'. \
                 Verify the field label and section name (if specified) are correct.",
                op_ref.field,
                op_ref.vault,
                op_ref.item,
            )
        })?;

    Ok(SecretString::from(field_value.to_string()))
}

/// Resolve a secret via the `op` CLI (`op read <reference>`).
///
/// This supports `OP_SERVICE_ACCOUNT_TOKEN` (non-interactive CI/CD) and
/// interactive `op signin` sessions (developer workstations).
///
/// A 30-second timeout is applied to prevent indefinite hangs if the `op`
/// CLI waits for biometric/GUI authentication or a stalled network call.
fn resolve_via_cli(reference: &str) -> Result<SecretString> {
    let mut child = match std::process::Command::new("op")
        .args(["read", "--no-newline", reference])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            bail!(
                "1Password secret resolution requires either:\n  \
                 1. Connect Server: set OP_CONNECT_TOKEN + OP_CONNECT_HOST\n  \
                 2. CLI: install `op` (https://developer.1password.com/docs/cli/) \
                 and set OP_SERVICE_ACCOUNT_TOKEN or run `op signin`"
            );
        }
        Err(err) => {
            bail!("failed to execute 1Password CLI `op read`: {err}");
        }
    };

    // Poll with timeout to avoid indefinite hangs (e.g., biometric prompts).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    child.kill().ok();
                    child.wait().ok();
                    bail!(
                        "1Password CLI `op read` did not complete within 30 seconds. \
                         The CLI may be waiting for interactive authentication (biometric/GUI). \
                         For non-interactive use, set OP_SERVICE_ACCOUNT_TOKEN or use Connect Server."
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(err) => bail!("failed to wait for 1Password CLI: {err}"),
        }
    }

    let result = child.wait_with_output()
        .context("failed to read 1Password CLI output")?;

    if result.status.success() {
        let value = String::from_utf8(result.stdout)
            .context("1Password CLI returned non-UTF-8 output")?;
        Ok(SecretString::from(value))
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!(
            "1Password CLI `op read` failed (exit {}): {}\n\n\
             Ensure you are authenticated via one of:\n  \
             - OP_SERVICE_ACCOUNT_TOKEN env var (CI/CD)\n  \
             - `op signin` interactive session (developer workstation)\n  \
             - Connect Server: set OP_CONNECT_TOKEN + OP_CONNECT_HOST",
            result.status.code().unwrap_or(-1),
            stderr.trim()
        );
    }
}

/// Resolve a vault name to its UUID via the Connect Server API.
fn resolve_vault_id(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    name_or_id: &str,
) -> Result<String> {
    // If the caller already supplied a UUID, skip the name-lookup entirely.
    // The SCIM filter `name eq "UUID"` searches display names, so a bare UUID
    // would match nothing and produce a confusing "vault not found" error.
    if is_op_uuid(name_or_id) {
        return Ok(name_or_id.to_string());
    }

    let request = client
        .get(format!("{base}/v1/vaults"))
        .query(&[("filter", format!("name eq \"{name_or_id}\""))])
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
            truncate_body(&body, ERROR_BODY_MAX_LEN)
        );
    }

    let vaults: Vec<serde_json::Value> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse 1Password vault lookup response")?;

    if vaults.is_empty() {
        bail!("1Password vault '{}' not found", name_or_id);
    }
    if vaults.len() > 1 {
        bail!(
            "1Password vault name '{}' is ambiguous — {} vaults matched. \
             Use the vault UUID instead to resolve the ambiguity.",
            name_or_id,
            vaults.len()
        );
    }

    let id = vaults[0]["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("1Password vault response missing 'id' field"))?;

    // Validate the returned ID is safe for URL interpolation (defense-in-depth).
    validate_op_segment(&id, "vault ID from API")?;

    Ok(id)
}

/// Resolve an item title to its UUID within a vault.
fn resolve_item_id(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    vault_id: &str,
    name_or_id: &str,
) -> Result<String> {
    // If the caller already supplied a UUID, skip the title-lookup entirely.
    if is_op_uuid(name_or_id) {
        return Ok(name_or_id.to_string());
    }

    let request = client
        .get(format!("{base}/v1/vaults/{vault_id}/items"))
        .query(&[("filter", format!("title eq \"{name_or_id}\""))])
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
            truncate_body(&body, ERROR_BODY_MAX_LEN)
        );
    }

    let items: Vec<serde_json::Value> = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse 1Password item lookup response")?;

    if items.is_empty() {
        bail!("1Password item '{}' not found in vault", name_or_id);
    }
    if items.len() > 1 {
        bail!(
            "1Password item title '{}' is ambiguous — {} items matched in vault. \
             Use the item UUID instead to resolve the ambiguity.",
            name_or_id,
            items.len()
        );
    }

    let id = items[0]["id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("1Password item response missing 'id' field"))?;

    // Validate the returned ID is safe for URL interpolation (defense-in-depth).
    validate_op_segment(&id, "item ID from API")?;

    Ok(id)
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
    fn parse_four_segment_reference() {
        let r = OpReference::parse("op://my-vault/my-item/login/password").unwrap();
        assert_eq!(r.vault, "my-vault");
        assert_eq!(r.item, "my-item");
        assert_eq!(r.field, "login/password");
    }

    #[test]
    fn parse_rejects_too_few_segments() {
        let err = OpReference::parse("op://vault/item").unwrap_err();
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn parse_rejects_too_many_segments() {
        let err = OpReference::parse("op://vault/item/a/b/c").unwrap_err();
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
    fn parse_accepts_spaces_in_vault() {
        // 1Password vault/item names commonly contain spaces.
        let r = OpReference::parse("op://My Vault/My Item/password").unwrap();
        assert_eq!(r.vault, "My Vault");
        assert_eq!(r.item, "My Item");
        assert_eq!(r.field, "password");
    }

    #[test]
    fn scim_validation_rejects_quotes() {
        let err = validate_scim_value("my\"vault", "vault name").unwrap_err();
        assert!(
            err.to_string().contains("SCIM filter injection"),
            "got: {err}"
        );
    }

    #[test]
    fn scim_validation_rejects_backslash() {
        let err = validate_scim_value("my\\vault", "vault name").unwrap_err();
        assert!(
            err.to_string().contains("SCIM filter injection"),
            "got: {err}"
        );
    }

    #[test]
    fn scim_validation_accepts_normal_names() {
        validate_scim_value("my-vault", "vault name").unwrap();
        validate_scim_value("My Vault 123", "vault name").unwrap();
    }

    #[test]
    fn connect_auth_requires_both_vars() {
        // Ensure OpAuth::from_env() fails when env vars are absent.
        // (This tests the branching that triggers CLI fallback.)
        // SAFETY: test-only, single-threaded access to env vars.
        unsafe { std::env::remove_var("OP_CONNECT_TOKEN") };
        unsafe { std::env::remove_var("OP_CONNECT_HOST") };
        assert!(OpAuth::from_env().is_err());
    }

    #[test]
    fn connect_auth_rejects_host_with_userinfo() {
        // SAFETY: test-only, single-threaded access to env vars.
        unsafe { std::env::set_var("OP_CONNECT_TOKEN", "tok") };
        unsafe { std::env::set_var("OP_CONNECT_HOST", "https://user:pass@op.example.com") };
        let err = OpAuth::from_env()
            .err()
            .expect("expected OpAuth::from_env() to fail");
        assert!(
            err.to_string().contains("userinfo"),
            "expected userinfo rejection, got: {err}"
        );
        unsafe { std::env::remove_var("OP_CONNECT_TOKEN") };
        unsafe { std::env::remove_var("OP_CONNECT_HOST") };
    }

    #[test]
    fn connect_auth_rejects_host_with_query() {
        // SAFETY: test-only, single-threaded access to env vars.
        unsafe { std::env::set_var("OP_CONNECT_TOKEN", "tok") };
        unsafe { std::env::set_var("OP_CONNECT_HOST", "https://op.example.com?foo=bar") };
        let err = OpAuth::from_env()
            .err()
            .expect("expected OpAuth::from_env() to fail");
        assert!(
            err.to_string().contains("query"),
            "expected query rejection, got: {err}"
        );
        unsafe { std::env::remove_var("OP_CONNECT_TOKEN") };
        unsafe { std::env::remove_var("OP_CONNECT_HOST") };
    }

    #[test]
    fn connect_auth_rejects_host_with_fragment() {
        // SAFETY: test-only, single-threaded access to env vars.
        unsafe { std::env::set_var("OP_CONNECT_TOKEN", "tok") };
        unsafe { std::env::set_var("OP_CONNECT_HOST", "https://op.example.com#section") };
        let err = OpAuth::from_env()
            .err()
            .expect("expected OpAuth::from_env() to fail");
        assert!(
            err.to_string().contains("fragment"),
            "expected fragment rejection, got: {err}"
        );
        unsafe { std::env::remove_var("OP_CONNECT_TOKEN") };
        unsafe { std::env::remove_var("OP_CONNECT_HOST") };
    }

    #[test]
    fn truncate_body_handles_multibyte_utf8() {
        // "😀" is 4 bytes (0xF0 0x9F 0x98 0x80). Truncating at byte 3 would
        // panic with a plain slice; our implementation should walk back safely.
        let s = "ab😀cd";
        // byte 3 is inside the 4-byte emoji — should walk back to byte 2.
        let truncated = truncate_body(s, 3);
        assert_eq!(truncated, "ab");
    }

    #[test]
    fn truncate_body_exact_boundary() {
        let s = "hello";
        assert_eq!(truncate_body(s, 5), "hello");
        assert_eq!(truncate_body(s, 3), "hel");
    }
}
