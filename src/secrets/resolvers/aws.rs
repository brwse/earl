use anyhow::{anyhow, bail, Context, Result};
use secrecy::SecretString;

use crate::secrets::resolver::SecretResolver;

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
pub struct AwsResolver;

impl AwsResolver {
    pub fn new() -> Self {
        Self
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

        // We are inside a sync trait method but need to perform async AWS SDK calls.
        // Use tokio's block_in_place + Handle::current().block_on() to bridge.
        let config = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(aws_config::load_defaults(aws_config::BehaviorVersion::latest()))
        });

        let client = aws_sdk_secretsmanager::Client::new(&config);

        let output = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(
                client
                    .get_secret_value()
                    .secret_id(&aws_ref.secret_id)
                    .send(),
            )
        })
        .with_context(|| {
            format!(
                "failed to retrieve AWS secret '{}'",
                aws_ref.secret_id
            )
        })?;

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
                    let available_keys: Vec<&str> = parsed
                        .as_object()
                        .map(|obj| obj.keys().map(|k| k.as_str()).collect())
                        .unwrap_or_default();
                    anyhow!(
                        "key '{}' not found in AWS secret '{}' (available keys: {})",
                        key,
                        aws_ref.secret_id,
                        available_keys.join(", ")
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
}
