use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use secrecy::SecretString;
use serde::Deserialize;

use crate::secrets::resolver::SecretResolver;
use crate::secrets::resolvers::validate_path_segment;

/// A parsed `gcp://project/secret-name` or `gcp://project/secret-name/version` reference.
#[derive(Debug)]
struct GcpReference {
    project: String,
    secret: String,
    version: String,
}

impl GcpReference {
    fn parse(reference: &str) -> Result<Self> {
        let after_scheme = reference
            .strip_prefix("gcp://")
            .ok_or_else(|| anyhow!("invalid GCP reference: must start with gcp://"))?;

        if after_scheme.is_empty() {
            bail!("invalid GCP reference: project and secret name are required in {reference}");
        }

        let segments: Vec<&str> = after_scheme.split('/').filter(|s| !s.is_empty()).collect();

        match segments.len() {
            0 | 1 => {
                bail!(
                    "invalid GCP reference: expected gcp://project/secret-name[/version], got: {reference}"
                );
            }
            2 => {
                let project = segments[0].to_string();
                let secret = segments[1].to_string();
                validate_path_segment(&project, "project name")?;
                validate_path_segment(&secret, "secret name")?;
                Ok(Self {
                    project,
                    secret,
                    version: "latest".to_string(),
                })
            }
            3 => {
                let project = segments[0].to_string();
                let secret = segments[1].to_string();
                let version = segments[2].to_string();
                validate_path_segment(&project, "project name")?;
                validate_path_segment(&secret, "secret name")?;
                validate_path_segment(&version, "version")?;
                Ok(Self {
                    project,
                    secret,
                    version,
                })
            }
            _ => {
                bail!(
                    "invalid GCP reference: too many path segments, expected gcp://project/secret-name[/version], got: {reference}"
                );
            }
        }
    }
}

/// Resolver for GCP Secret Manager secrets using the `gcp://` URI scheme.
///
/// Authentication uses Application Default Credentials (ADC) with the following
/// precedence:
///
/// 1. `GOOGLE_APPLICATION_CREDENTIALS` env var pointing to a service account or
///    user credentials JSON file
/// 2. Well-known user credentials at `~/.config/gcloud/application_default_credentials.json`
/// 3. GCE metadata server (for workloads running on Google Cloud)
///
/// References use one of two formats:
/// * `gcp://project/secret-name` — accesses the `latest` version
/// * `gcp://project/secret-name/version` — accesses a specific version
///
/// Example: `gcp://my-project/api-key` or `gcp://my-project/api-key/3`
pub struct GcpResolver;

impl GcpResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GcpResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretResolver for GcpResolver {
    fn scheme(&self) -> &str {
        "gcp"
    }

    fn resolve(&self, reference: &str) -> Result<SecretString> {
        let gcp_ref = GcpReference::parse(reference)?;

        let access_token = obtain_access_token()
            .context("failed to obtain GCP access token")?;

        let url = format!(
            "https://secretmanager.googleapis.com/v1/projects/{}/secrets/{}/versions/{}:access",
            gcp_ref.project, gcp_ref.secret, gcp_ref.version
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to build HTTP client for GCP Secret Manager")?;

        let request = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .build()
            .context("failed to build GCP Secret Manager request")?;

        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.execute(request))
        })
        .context("GCP Secret Manager API request failed")?;

        let status = response.status();
        if !status.is_success() {
            let body = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(response.text())
            })
            .unwrap_or_default();
            bail!(
                "GCP Secret Manager API returned HTTP {}: {}",
                status.as_u16(),
                body
            );
        }

        let body: SecretAccessResponse = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.json())
        })
        .context("failed to parse GCP Secret Manager API response")?;

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&body.payload.data)
            .context("failed to base64-decode secret payload")?;

        let secret_value = String::from_utf8(decoded)
            .context("GCP secret payload is not valid UTF-8")?;

        Ok(SecretString::from(secret_value))
    }
}

/// Response from the Secret Manager `accessSecretVersion` endpoint.
#[derive(Deserialize)]
struct SecretAccessResponse {
    payload: SecretPayload,
}

/// The payload field within the access response.
#[derive(Deserialize)]
struct SecretPayload {
    data: String,
}

/// Token response from the OAuth2 token endpoint or metadata server.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Service account credentials JSON file.
#[derive(Deserialize)]
struct ServiceAccountCredentials {
    client_email: String,
    private_key: String,
    token_uri: Option<String>,
}

/// User (authorized_user) credentials JSON file.
#[derive(Deserialize)]
struct UserCredentials {
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

/// Generic credentials file — we inspect `type` to determine the variant.
#[derive(Deserialize)]
struct CredentialsFile {
    r#type: String,
}

// ---------------------------------------------------------------------------
// Application Default Credentials (ADC)
// ---------------------------------------------------------------------------

/// Obtain an access token using Application Default Credentials.
fn obtain_access_token() -> Result<String> {
    // 1. Check GOOGLE_APPLICATION_CREDENTIALS env var
    if let Ok(path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        && !path.is_empty()
    {
        let creds_path = std::path::Path::new(&path);
        if creds_path.exists() {
            return token_from_credentials_file(creds_path);
        }
    }

    // 2. Check well-known user credentials location
    let well_known = well_known_credentials_path();
    if let Some(ref path) = well_known
        && path.exists()
    {
        return token_from_credentials_file(path);
    }

    // 3. Try GCE metadata server
    if let Ok(token) = token_from_metadata_server() {
        return Ok(token);
    }

    bail!(
        "GCP credentials not found. Set GOOGLE_APPLICATION_CREDENTIALS to a service account \
         key file, or run `gcloud auth application-default login` to create user credentials."
    );
}

/// Returns the well-known ADC credentials path.
fn well_known_credentials_path() -> Option<std::path::PathBuf> {
    // On macOS/Linux: ~/.config/gcloud/application_default_credentials.json
    // On Windows: %APPDATA%/gcloud/application_default_credentials.json
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|appdata| {
            std::path::PathBuf::from(appdata)
                .join("gcloud")
                .join("application_default_credentials.json")
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()));
        home.map(|h| {
            h.join(".config")
                .join("gcloud")
                .join("application_default_credentials.json")
        })
    }
}

/// Load and process a credentials JSON file (service account or authorized_user).
fn token_from_credentials_file(path: &std::path::Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read credentials file at {}", path.display()))?;

    let creds_file: CredentialsFile = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse credentials file at {}", path.display()))?;

    match creds_file.r#type.as_str() {
        "service_account" => {
            let sa: ServiceAccountCredentials = serde_json::from_str(&content)
                .context("failed to parse service account credentials")?;
            token_from_service_account(&sa)
        }
        "authorized_user" => {
            let user: UserCredentials = serde_json::from_str(&content)
                .context("failed to parse user credentials")?;
            token_from_user_credentials(&user)
        }
        other => bail!(
            "unsupported GCP credentials type '{}' in {}",
            other,
            path.display()
        ),
    }
}

/// Exchange a service account key for an access token using a self-signed JWT.
fn token_from_service_account(sa: &ServiceAccountCredentials) -> Result<String> {
    let token_uri = sa
        .token_uri
        .as_deref()
        .unwrap_or("https://oauth2.googleapis.com/token");

    let now = chrono::Utc::now().timestamp();
    let claims = serde_json::json!({
        "iss": sa.client_email,
        "scope": "https://www.googleapis.com/auth/cloud-platform",
        "aud": token_uri,
        "iat": now,
        "exp": now + 3600,
    });

    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .context("failed to parse service account private key")?;
    let jwt = jsonwebtoken::encode(&header, &claims, &key)
        .context("failed to sign JWT for service account")?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client for token exchange")?;

    let request = client
        .post(token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .build()
        .context("failed to build token exchange request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("service account token exchange request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.text())
        })
        .unwrap_or_default();
        bail!(
            "GCP token exchange returned HTTP {}: {}",
            status.as_u16(),
            body
        );
    }

    let token_resp: TokenResponse = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse token exchange response")?;

    Ok(token_resp.access_token)
}

/// Exchange a refresh token for an access token (authorized_user credentials).
fn token_from_user_credentials(user: &UserCredentials) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client for token refresh")?;

    let request = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", user.client_id.as_str()),
            ("client_secret", user.client_secret.as_str()),
            ("refresh_token", user.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .build()
        .context("failed to build token refresh request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("user credentials token refresh request failed")?;

    let status = response.status();
    if !status.is_success() {
        let body = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(response.text())
        })
        .unwrap_or_default();
        bail!(
            "GCP token refresh returned HTTP {}: {}",
            status.as_u16(),
            body
        );
    }

    let token_resp: TokenResponse = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse token refresh response")?;

    Ok(token_resp.access_token)
}

/// Try to obtain an access token from the GCE metadata server.
/// Uses a short 2-second timeout since this will fail fast on non-GCE machines.
fn token_from_metadata_server() -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .context("failed to build HTTP client for metadata server")?;

    let request = client
        .get("http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token")
        .header("Metadata-Flavor", "Google")
        .build()
        .context("failed to build metadata server request")?;

    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.execute(request))
    })
    .context("GCE metadata server request failed")?;

    let status = response.status();
    if !status.is_success() {
        bail!("GCE metadata server returned HTTP {}", status.as_u16());
    }

    let token_resp: TokenResponse = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(response.json())
    })
    .context("failed to parse metadata server token response")?;

    Ok(token_resp.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_project_and_secret() {
        let r = GcpReference::parse("gcp://my-project/my-secret").unwrap();
        assert_eq!(r.project, "my-project");
        assert_eq!(r.secret, "my-secret");
        assert_eq!(r.version, "latest");
    }

    #[test]
    fn parse_with_explicit_version() {
        let r = GcpReference::parse("gcp://my-project/my-secret/42").unwrap();
        assert_eq!(r.project, "my-project");
        assert_eq!(r.secret, "my-secret");
        assert_eq!(r.version, "42");
    }

    #[test]
    fn parse_rejects_empty() {
        let err = GcpReference::parse("gcp://").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_project_only() {
        let err = GcpReference::parse("gcp://my-project").unwrap_err();
        assert!(
            err.to_string().contains("invalid") || err.to_string().contains("expected"),
            "got: {}",
            err
        );
    }

    #[test]
    fn parse_rejects_too_many_segments() {
        let err = GcpReference::parse("gcp://project/secret/version/extra").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        let err = GcpReference::parse("aws://project/secret").unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {}", err);
    }

    #[test]
    fn parse_rejects_question_mark_in_project() {
        let err = GcpReference::parse("gcp://proj?ect/secret").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_hash_in_secret() {
        let err = GcpReference::parse("gcp://project/sec#ret").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_whitespace_in_project() {
        let err = GcpReference::parse("gcp://my project/secret").unwrap_err();
        assert!(
            err.to_string().contains("invalid character"),
            "got: {err}"
        );
    }
}
