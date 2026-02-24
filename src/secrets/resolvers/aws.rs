use std::sync::Mutex;

use anyhow::{anyhow, bail, Context, Result};
use aws_sdk_secretsmanager::error::ProvideErrorMetadata;
use secrecy::SecretString;

use crate::secrets::resolver::SecretResolver;

/// Characters that are unsafe in AWS secret references (query/fragment delimiters).
const UNSAFE_CHARS: &[char] = &['?', '#'];

/// Validate that an AWS reference component does not contain `?`, `#`,
/// whitespace, or control characters. Slashes are allowed in secret names.
fn validate_aws_component(value: &str, field_name: &str) -> Result<()> {
    for ch in value.chars() {
        if UNSAFE_CHARS.contains(&ch) || ch.is_whitespace() || ch.is_control() {
            bail!(
                "{field_name} contains invalid character '{}' — \
                 must not contain '?', '#', whitespace, or control characters",
                ch.escape_debug()
            );
        }
    }
    Ok(())
}

/// A parsed `aws://secret-name` or `aws://secret-name#json-key` reference.
#[derive(Debug)]
struct AwsReference {
    secret_id: String,
    json_key: Option<String>,
}

impl AwsReference {
    fn parse(reference: &str) -> Result<Self> {
        let after_scheme = reference
            .strip_prefix("aws://")
            .ok_or_else(|| anyhow!("invalid AWS reference: must start with aws://"))?;

        if after_scheme.is_empty() {
            bail!("invalid AWS reference: secret name must not be empty in {reference}");
        }

        // Split on '#' to separate secret name from optional JSON key
        let (secret_id, json_key) = match after_scheme.split_once('#') {
            Some((name, key)) => {
                if name.is_empty() {
                    bail!("invalid AWS reference: secret name must not be empty in {reference}");
                }
                if key.is_empty() {
                    bail!("invalid AWS reference: JSON key after '#' must not be empty in {reference}");
                }
                (name.to_string(), Some(key.to_string()))
            }
            None => (after_scheme.to_string(), None),
        };

        validate_aws_component(&secret_id, "secret name")?;
        if let Some(ref key) = json_key {
            validate_aws_component(key, "JSON key")?;
        }

        Ok(Self {
            secret_id,
            json_key,
        })
    }
}

/// Resolver for AWS Secrets Manager secrets using the `aws://` URI scheme.
///
/// Authentication is handled by the standard AWS SDK credential chain (environment
/// variables, `~/.aws/credentials`, IAM role, etc.).
///
/// References use one of two formats:
/// * `aws://secret-name` — returns the full `SecretString`
/// * `aws://secret-name#json-key` — parses `SecretString` as JSON and extracts the key
///
/// Examples:
/// * `aws://prod/api-key` — returns the raw secret value
/// * `aws://prod/db-creds#password` — parses the secret as JSON and returns the `password` field
///
/// **Note on retries:** The AWS SDK has its own internal retry logic (3 attempts by
/// default). Earl's retry wrapper in `require_secret()` adds an additional layer,
/// so AWS calls may see up to 9 total attempts on transient failures.
pub struct AwsResolver {
    /// Cached AWS SDK client — reused across resolves to avoid re-loading
    /// credentials and config on every call.
    client_cache: Mutex<Option<aws_sdk_secretsmanager::Client>>,
}

impl AwsResolver {
    pub fn new() -> Self {
        Self {
            client_cache: Mutex::new(None),
        }
    }
}

impl Default for AwsResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::result_large_err)]
impl SecretResolver for AwsResolver {
    fn scheme(&self) -> &str {
        "aws"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let aws_ref = AwsReference::parse(reference)?;

        // Reuse cached client to avoid re-loading credentials/config per call.
        let client = {
            let mut cache = self.client_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref client) = *cache {
                client.clone()
            } else {
                let config = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(aws_config::load_defaults(aws_config::BehaviorVersion::latest()))
                });
                let new_client = aws_sdk_secretsmanager::Client::new(&config);
                *cache = Some(new_client.clone());
                new_client
            }
        };

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                client
                    .get_secret_value()
                    .secret_id(&aws_ref.secret_id)
                    .send(),
            )
        });

        let output = match result {
            Ok(output) => output,
            Err(err) => {
                // Use typed service error matching where possible for stability.
                if let Some(svc_err) = err.as_service_error() {
                    // ResourceNotFoundException: the secret name does not exist in
                    // Secrets Manager (or not in the configured region).
                    if svc_err.is_resource_not_found_exception() {
                        bail!(
                            "AWS secret '{}' was not found in Secrets Manager. \
                             Verify the secret name and that AWS_REGION or \
                             AWS_DEFAULT_REGION points to the correct region.",
                            aws_ref.secret_id
                        );
                    }
                    // InvalidRequestException: secret exists but the request is
                    // invalid — most commonly because the secret is scheduled for
                    // deletion (pending delete) or is in a state that blocks retrieval.
                    if svc_err.is_invalid_request_exception() {
                        bail!(
                            "AWS secret '{}' cannot be retrieved: the secret may be \
                             scheduled for deletion or in an invalid state. \
                             Check the secret status in the AWS console.",
                            aws_ref.secret_id
                        );
                    }
                    // DecryptionFailure: the KMS key used to encrypt the secret
                    // could not be used for decryption (distinct from IAM AccessDenied).
                    if svc_err.is_decryption_failure() {
                        bail!(
                            "AWS secret '{}' could not be decrypted: the KMS key used to \
                             encrypt this secret is unavailable, disabled, or the IAM principal \
                             lacks kms:Decrypt permission.",
                            aws_ref.secret_id
                        );
                    }
                    // InvalidParameterException: the request contained an invalid value
                    // (e.g., a malformed secret name or unsupported parameter combination).
                    if svc_err.is_invalid_parameter_exception() {
                        bail!(
                            "AWS secret '{}': invalid parameter — verify the secret name is \
                             correct and contains no unsupported characters.",
                            aws_ref.secret_id
                        );
                    }
                    // AccessDeniedException is not a modeled error for GetSecretValue;
                    // it surfaces as an unhandled service error with a known error code.
                    // Note: SdkError's Display ignores f.alternate(), so format!("{err:#}")
                    // always produces "service error" and cannot be used for string matching.
                    if svc_err.code() == Some("AccessDeniedException") {
                        bail!(
                            "IAM access denied for AWS secret '{}': the IAM principal lacks \
                             secretsmanager:GetSecretValue permission (or kms:Decrypt if using \
                             a customer-managed KMS key). Verify the IAM policy grants access \
                             to this secret.",
                            aws_ref.secret_id
                        );
                    }
                }
                return Err(anyhow::anyhow!(err).context(format!(
                    "failed to retrieve AWS secret '{}': ensure AWS credentials are configured \
                     (AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY/AWS_SESSION_TOKEN, \
                     ~/.aws/credentials, or IAM role). \
                     Set AWS_REGION or AWS_DEFAULT_REGION to your secret's region.",
                    aws_ref.secret_id
                )));
            }
        };

        let secret_string = output
            .secret_string()
            .ok_or_else(|| {
                anyhow!(
                    "AWS secret '{}' does not contain a SecretString (it may be binary)",
                    aws_ref.secret_id
                )
            })?;

        match &aws_ref.json_key {
            Some(key) => {
                let parsed: serde_json::Value = serde_json::from_str(secret_string)
                    .with_context(|| {
                        format!(
                            "failed to parse AWS secret '{}' as JSON (needed for '#{}' key extraction)",
                            aws_ref.secret_id, key
                        )
                    })?;

                let value = parsed.get(key).ok_or_else(|| {
                    anyhow!(
                        "top-level key '{}' not found in AWS secret '{}'. \
                         Verify the key exists at the top level of the secret's JSON structure. \
                         Note: nested key paths (e.g., 'a.b') are not supported — \
                         only top-level keys can be extracted.",
                        key,
                        aws_ref.secret_id,
                    )
                })?;

                // Extract the string value — if it's a JSON string, unwrap it;
                // otherwise use its JSON representation.
                let text = match value.as_str() {
                    Some(s) => s.to_string(),
                    None => value.to_string(),
                };

                Ok(SecretString::from(text))
            }
            None => Ok(SecretString::from(secret_string.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_name() {
        let r = AwsReference::parse("aws://my-secret").unwrap();
        assert_eq!(r.secret_id, "my-secret");
        assert!(r.json_key.is_none());
    }

    #[test]
    fn parse_name_with_slashes() {
        let r = AwsReference::parse("aws://prod/db/credentials").unwrap();
        assert_eq!(r.secret_id, "prod/db/credentials");
        assert!(r.json_key.is_none());
    }

    #[test]
    fn parse_name_with_json_key() {
        let r = AwsReference::parse("aws://prod/db-creds#password").unwrap();
        assert_eq!(r.secret_id, "prod/db-creds");
        assert_eq!(r.json_key.as_deref(), Some("password"));
    }

    #[test]
    fn parse_rejects_empty_name() {
        let err = AwsReference::parse("aws://").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_empty_name_with_key() {
        let err = AwsReference::parse("aws://#key").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_empty_key() {
        let err = AwsReference::parse("aws://secret#").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let err = AwsReference::parse("vault://secret/path#field").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_question_mark_in_secret_id() {
        let err = AwsReference::parse("aws://my-secret?inject=1").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_control_char_in_secret_id() {
        let err = AwsReference::parse("aws://my\x00secret").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_whitespace_in_json_key() {
        let err = AwsReference::parse("aws://secret#my key").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }
}
