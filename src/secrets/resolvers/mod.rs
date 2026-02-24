#[cfg(feature = "secrets-1password")]
pub mod onepassword;

#[cfg(feature = "secrets-vault")]
pub mod vault;

#[cfg(feature = "secrets-aws")]
pub mod aws;

#[cfg(feature = "secrets-gcp")]
pub mod gcp;

#[cfg(feature = "secrets-azure")]
pub mod azure;

#[cfg(any(
    feature = "secrets-vault",
    feature = "secrets-gcp",
    feature = "secrets-azure",
))]
use anyhow::{bail, Result};

/// Cached bearer token with expiry tracking.
///
/// Used by providers that perform token exchange (GCP, Azure) to avoid
/// redundant token requests within a single CLI invocation.
#[cfg(any(feature = "secrets-gcp", feature = "secrets-azure"))]
pub(crate) struct CachedToken {
    pub token: String,
    pub expires_at: std::time::Instant,
}

#[cfg(any(feature = "secrets-gcp", feature = "secrets-azure"))]
impl CachedToken {
    /// Returns the token if it's still valid (with 30-second safety margin).
    pub fn get_if_valid(&self) -> Option<&str> {
        if self.expires_at > std::time::Instant::now() + std::time::Duration::from_secs(30) {
            Some(&self.token)
        } else {
            None
        }
    }
}

/// Characters that are unsafe in URL path segments.
#[cfg(any(
    feature = "secrets-vault",
    feature = "secrets-gcp",
    feature = "secrets-azure",
))]
const UNSAFE_PATH_CHARS: &[char] = &['/', '?', '#'];

/// Validate that a value is safe to use in a URL path segment.
///
/// Rejects values containing `/`, `?`, `#`, whitespace, and control characters
/// which could break or manipulate URL construction.
#[cfg(any(
    feature = "secrets-vault",
    feature = "secrets-gcp",
    feature = "secrets-azure",
))]
pub(crate) fn validate_path_segment(value: &str, field_name: &str) -> Result<()> {
    if value.is_empty() {
        bail!("{field_name} must not be empty");
    }

    for ch in value.chars() {
        if UNSAFE_PATH_CHARS.contains(&ch) || ch.is_whitespace() || ch.is_control() {
            bail!(
                "{field_name} contains invalid character '{}' — \
                 must not contain '/', '?', '#', whitespace, or control characters",
                ch.escape_debug()
            );
        }
    }

    Ok(())
}

/// Truncate a response body for error messages to avoid emitting multi-kilobyte
/// HTML error pages (common for 5xx responses from API gateways) to the terminal.
#[cfg(any(
    feature = "secrets-1password",
    feature = "secrets-azure",
    feature = "secrets-gcp",
))]
pub(crate) fn truncate_body(body: &str, max_len: usize) -> &str {
    if body.len() <= max_len {
        body
    } else {
        let mut end = max_len;
        // Walk back to a valid UTF-8 character boundary to avoid panicking
        // on multi-byte character sequences (e.g., 3-byte UTF-8 emoji).
        while end > 0 && !body.is_char_boundary(end) {
            end -= 1;
        }
        &body[..end]
    }
}

/// Truncation limit for HTTP error response bodies in error messages.
#[cfg(any(
    feature = "secrets-1password",
    feature = "secrets-azure",
    feature = "secrets-gcp",
))]
pub(crate) const ERROR_BODY_MAX_LEN: usize = 256;

/// Validate an Azure Key Vault name.
///
/// Azure vault names must be 3-24 characters, containing only alphanumeric
/// characters and hyphens. They must not start or end with a hyphen, and must
/// not contain consecutive hyphens.
#[cfg(feature = "secrets-azure")]
pub(crate) fn validate_azure_vault_name(name: &str) -> Result<()> {
    if name.len() < 3 || name.len() > 24 {
        bail!(
            "Azure vault name must be 3-24 characters long, got {} characters",
            name.len()
        );
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        bail!(
            "Azure vault name must contain only alphanumeric characters and hyphens, got: {name}"
        );
    }

    if name.starts_with('-') || name.ends_with('-') {
        bail!("Azure vault name must not start or end with a hyphen");
    }

    if name.contains("--") {
        bail!("Azure vault name must not contain consecutive hyphens");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_accepts_valid() {
        super::validate_path_segment("my-vault", "vault").unwrap();
        super::validate_path_segment("my_item.name", "item").unwrap();
        super::validate_path_segment("123", "version").unwrap();
    }

    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_rejects_slash() {
        let err = super::validate_path_segment("foo/bar", "field").unwrap_err();
        assert!(err.to_string().contains("invalid character"), "got: {err}");
    }

    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_rejects_question_mark() {
        let err = super::validate_path_segment("foo?bar", "field").unwrap_err();
        assert!(err.to_string().contains("invalid character"), "got: {err}");
    }

    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_rejects_hash() {
        let err = super::validate_path_segment("foo#bar", "field").unwrap_err();
        assert!(err.to_string().contains("invalid character"), "got: {err}");
    }

    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_rejects_whitespace() {
        let err = super::validate_path_segment("foo bar", "field").unwrap_err();
        assert!(err.to_string().contains("invalid character"), "got: {err}");
    }

    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_rejects_control_chars() {
        let err = super::validate_path_segment("foo\x00bar", "field").unwrap_err();
        assert!(err.to_string().contains("invalid character"), "got: {err}");
    }

    #[test]
    #[cfg(any(
        feature = "secrets-vault",
        feature = "secrets-gcp",
        feature = "secrets-azure",
    ))]
    fn validate_path_segment_rejects_empty() {
        let err = super::validate_path_segment("", "field").unwrap_err();
        assert!(err.to_string().contains("must not be empty"), "got: {err}");
    }

    #[test]
    #[cfg(feature = "secrets-azure")]
    fn validate_azure_vault_name_accepts_valid() {
        super::validate_azure_vault_name("my-vault").unwrap();
        super::validate_azure_vault_name("abc").unwrap();
        super::validate_azure_vault_name("vault123").unwrap();
    }

    #[test]
    #[cfg(feature = "secrets-azure")]
    fn validate_azure_vault_name_rejects_too_short() {
        let err = super::validate_azure_vault_name("ab").unwrap_err();
        assert!(err.to_string().contains("3-24"), "got: {err}");
    }

    #[test]
    #[cfg(feature = "secrets-azure")]
    fn validate_azure_vault_name_rejects_too_long() {
        let err = super::validate_azure_vault_name("a".repeat(25).as_str()).unwrap_err();
        assert!(err.to_string().contains("3-24"), "got: {err}");
    }

    #[test]
    #[cfg(feature = "secrets-azure")]
    fn validate_azure_vault_name_rejects_dots() {
        let err = super::validate_azure_vault_name("my.vault").unwrap_err();
        assert!(
            err.to_string().contains("alphanumeric"),
            "got: {err}"
        );
    }

    #[test]
    #[cfg(feature = "secrets-azure")]
    fn validate_azure_vault_name_rejects_leading_hyphen() {
        let err = super::validate_azure_vault_name("-vault").unwrap_err();
        assert!(
            err.to_string().contains("must not start or end"),
            "got: {err}"
        );
    }

    #[test]
    #[cfg(feature = "secrets-azure")]
    fn validate_azure_vault_name_rejects_consecutive_hyphens() {
        let err = super::validate_azure_vault_name("my--vault").unwrap_err();
        assert!(
            err.to_string().contains("consecutive hyphens"),
            "got: {err}"
        );
    }
}
