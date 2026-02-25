use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

use crate::config::JwtConfig;
use crate::security::dns::resolve_and_validate_host;

const MAX_KID_LENGTH: usize = 256;
const MAX_JWKS_RESPONSE_BYTES: usize = 1_048_576; // 1 MiB
const KID_MISS_REFRESH_COOLDOWN: Duration = Duration::from_secs(30);
const STALE_GRACE_PERIOD: Duration = Duration::from_secs(300); // 5 minutes

/// The authenticated subject extracted from the JWT `sub` claim.
#[derive(Clone, Serialize)]
pub struct Subject(
    pub String,
    #[serde(skip_serializing_if = "Option::is_none")] pub Option<String>,
);

impl Subject {
    /// Return the first 8 characters of the subject, or the full value if shorter.
    fn redacted(&self) -> &str {
        match self.0.char_indices().nth(8) {
            Some((byte_pos, _)) => &self.0[..byte_pos],
            None => &self.0,
        }
    }
}

impl fmt::Debug for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chars = self.0.chars().count();
        if chars > 8 {
            f.debug_tuple("Subject")
                .field(&format_args!("{}...", self.redacted()))
                .finish()
        } else {
            f.debug_tuple("Subject").field(&self.0).finish()
        }
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let chars = self.0.chars().count();
        if chars > 8 {
            write!(f, "{}...", self.redacted())
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

/// A single JWK key. We store the full raw JSON so that `DecodingKey::from_jwk`
/// receives all fields (including `kty`, `kid`, `n`, `e`, etc.).
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
struct JwkKey {
    raw: serde_json::Value,
}

impl JwkKey {
    fn kid(&self) -> Option<&str> {
        self.raw.get("kid").and_then(|v| v.as_str())
    }

    fn kty(&self) -> Option<&str> {
        self.raw.get("kty").and_then(|v| v.as_str())
    }
}

#[derive(Debug)]
struct JwksCache {
    keys: Vec<JwkKey>,
    fetched_at: Instant,
    last_kid_miss_refresh: Option<Instant>,
}

#[derive(Debug, Clone, Deserialize)]
struct OidcDiscovery {
    issuer: String,
    jwks_uri: String,
}

/// Resolved JWT configuration (after OIDC discovery, if applicable).
#[derive(Debug, Clone)]
pub struct ResolvedJwtConfig {
    pub issuer: String,
    pub jwks_uri: String,
    pub audience: String,
    pub algorithms: Vec<Algorithm>,
    pub clock_skew_seconds: u64,
    pub jwks_cache_max_age_seconds: u64,
}

#[derive(Clone)]
pub struct JwtState {
    config: ResolvedJwtConfig,
    cache: Arc<RwLock<JwksCache>>,
    http: reqwest::Client,
}

impl JwtState {
    /// Resolve the JWT configuration (fetch OIDC discovery if needed) and build state.
    pub async fn new(jwt_config: &JwtConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed building JWKS HTTP client")?;

        let (issuer, jwks_uri) = if let Some(discovery_url) = &jwt_config.oidc_discovery_url {
            let url = url::Url::parse(discovery_url)
                .with_context(|| format!("invalid oidc_discovery_url: {discovery_url}"))?;
            let host = url.host_str().context("oidc_discovery_url has no host")?;
            resolve_and_validate_host(host).await?;

            let resp =
                http.get(discovery_url).send().await.with_context(|| {
                    format!("failed fetching OIDC discovery from {discovery_url}")
                })?;
            let body = resp
                .bytes()
                .await
                .context("failed reading OIDC discovery response")?;
            if body.len() > MAX_JWKS_RESPONSE_BYTES {
                bail!("OIDC discovery response exceeds 1 MiB limit");
            }
            let discovery: OidcDiscovery =
                serde_json::from_slice(&body).context("failed parsing OIDC discovery document")?;
            (discovery.issuer, discovery.jwks_uri)
        } else {
            let issuer = jwt_config
                .issuer
                .clone()
                .context("[auth.jwt] requires either oidc_discovery_url or issuer")?;
            let jwks_uri = jwt_config
                .jwks_uri
                .clone()
                .context("[auth.jwt] requires either oidc_discovery_url or jwks_uri")?;
            (issuer, jwks_uri)
        };

        // SSRF-validate the JWKS URI
        let jwks_url =
            url::Url::parse(&jwks_uri).with_context(|| format!("invalid jwks_uri: {jwks_uri}"))?;
        let jwks_host = jwks_url.host_str().context("jwks_uri has no host")?;
        resolve_and_validate_host(jwks_host).await?;

        let algorithms: Vec<Algorithm> = jwt_config
            .algorithms
            .iter()
            .map(|alg| match alg.to_uppercase().as_str() {
                "RS256" => Ok(Algorithm::RS256),
                "RS384" => Ok(Algorithm::RS384),
                "RS512" => Ok(Algorithm::RS512),
                "ES256" => Ok(Algorithm::ES256),
                "ES384" => Ok(Algorithm::ES384),
                "PS256" => Ok(Algorithm::PS256),
                "PS384" => Ok(Algorithm::PS384),
                "PS512" => Ok(Algorithm::PS512),
                "EDDSA" => Ok(Algorithm::EdDSA),
                other => bail!("unsupported JWT algorithm: {other}"),
            })
            .collect::<Result<_>>()?;

        if algorithms.is_empty() {
            bail!("[auth.jwt] algorithms must not be empty");
        }

        let clock_skew = jwt_config.clock_skew_seconds.min(300);

        let config = ResolvedJwtConfig {
            issuer,
            jwks_uri,
            audience: jwt_config.audience.clone(),
            algorithms,
            clock_skew_seconds: clock_skew,
            jwks_cache_max_age_seconds: jwt_config.jwks_cache_max_age_seconds,
        };

        let cache = Arc::new(RwLock::new(JwksCache {
            keys: Vec::new(),
            fetched_at: Instant::now() - Duration::from_secs(config.jwks_cache_max_age_seconds + 1),
            last_kid_miss_refresh: None,
        }));

        Ok(Self {
            config,
            cache,
            http,
        })
    }

    async fn fetch_jwks(&self) -> Result<Vec<JwkKey>> {
        // SSRF-validate on every fetch (DNS rebinding protection)
        let url = url::Url::parse(&self.config.jwks_uri)?;
        let host = url.host_str().context("jwks_uri has no host")?;
        resolve_and_validate_host(host).await?;

        let resp = self
            .http
            .get(&self.config.jwks_uri)
            .send()
            .await
            .with_context(|| format!("failed fetching JWKS from {}", self.config.jwks_uri))?;

        if !resp.status().is_success() {
            bail!("JWKS endpoint returned status {}", resp.status());
        }

        let body = resp.bytes().await.context("failed reading JWKS response")?;
        if body.len() > MAX_JWKS_RESPONSE_BYTES {
            bail!("JWKS response exceeds 1 MiB limit");
        }

        let jwks: JwksResponse =
            serde_json::from_slice(&body).context("failed parsing JWKS response")?;
        Ok(jwks.keys)
    }

    async fn get_or_refresh_keys(&self, kid: Option<&str>) -> Result<Vec<JwkKey>> {
        let now = Instant::now();
        // Per-process jitter (60-90s) to prevent thundering herd across instances
        let jitter_secs = (std::process::id() as u64 % 31) + 60;
        let max_age = Duration::from_secs(self.config.jwks_cache_max_age_seconds)
            + Duration::from_secs(jitter_secs);

        // Check if cache is fresh
        {
            let cache = self.cache.read().await;
            let age = now.duration_since(cache.fetched_at);
            if age < max_age && !cache.keys.is_empty() {
                // If we have the kid or no kid was requested, use cache
                if kid.is_none()
                    || kid.is_some_and(|k| cache.keys.iter().any(|key| key.kid() == Some(k)))
                {
                    return Ok(cache.keys.clone());
                }
                // Kid miss -- check cooldown
                if let Some(last_refresh) = cache.last_kid_miss_refresh
                    && now.duration_since(last_refresh) < KID_MISS_REFRESH_COOLDOWN
                {
                    return Ok(cache.keys.clone()); // Return stale, don't re-fetch
                }
            }
        }

        // Need to refresh
        match self.fetch_jwks().await {
            Ok(keys) => {
                let mut cache = self.cache.write().await;
                cache.keys = keys.clone();
                cache.fetched_at = Instant::now();
                if kid.is_some() {
                    cache.last_kid_miss_refresh = Some(Instant::now());
                }
                Ok(keys)
            }
            Err(err) => {
                // Serve stale keys during grace period
                let cache = self.cache.read().await;
                let staleness = now.duration_since(cache.fetched_at);
                if staleness < max_age + STALE_GRACE_PERIOD && !cache.keys.is_empty() {
                    tracing::warn!(
                        "JWKS refresh failed, serving stale keys (age: {}s): {err}",
                        staleness.as_secs()
                    );
                    Ok(cache.keys.clone())
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Verify a JWT and return the Subject.
    pub async fn verify_token(&self, token: &str) -> Result<Subject, AuthError> {
        let header = decode_header(token).map_err(|e| {
            tracing::debug!("JWT header decode failed: {e}");
            AuthError::Invalid("failed to decode token header")
        })?;

        // Validate kid length
        if let Some(kid) = &header.kid
            && kid.len() > MAX_KID_LENGTH
        {
            return Err(AuthError::Invalid("kid exceeds maximum length"));
        }

        // Verify algorithm is in our allowed list
        if !self.config.algorithms.contains(&header.alg) {
            tracing::debug!(
                "JWT uses algorithm {:?}, allowed: {:?}",
                header.alg,
                self.config.algorithms
            );
            return Err(AuthError::Invalid("token uses disallowed algorithm"));
        }

        let keys = self
            .get_or_refresh_keys(header.kid.as_deref())
            .await
            .map_err(|e| {
                tracing::debug!("JWKS fetch failed: {e}");
                AuthError::Invalid("failed to fetch signing keys")
            })?;

        // Find the matching key
        let jwk = if let Some(kid) = &header.kid {
            keys.iter().find(|k| k.kid() == Some(kid.as_str()))
        } else {
            keys.first()
        };

        let jwk = jwk.ok_or_else(|| {
            tracing::debug!("No matching key found for kid: {:?}", header.kid);
            AuthError::Invalid("no matching signing key found")
        })?;

        // Validate key type matches algorithm family
        let expected_kty = match header.alg {
            Algorithm::RS256
            | Algorithm::RS384
            | Algorithm::RS512
            | Algorithm::PS256
            | Algorithm::PS384
            | Algorithm::PS512 => "RSA",
            Algorithm::ES256 | Algorithm::ES384 => "EC",
            Algorithm::EdDSA => "OKP",
            _ => return Err(AuthError::Invalid("unsupported algorithm")),
        };
        let kty = jwk.kty().unwrap_or("");
        if kty.to_uppercase() != expected_kty {
            tracing::debug!("Key type mismatch: expected {expected_kty}, got {kty}");
            return Err(AuthError::Invalid("signing key type mismatch"));
        }

        let decoding_key =
            DecodingKey::from_jwk(&serde_json::from_value(jwk.raw.clone()).map_err(|e| {
                tracing::debug!("Failed to parse JWK: {e}");
                AuthError::Invalid("failed to parse signing key")
            })?)
            .map_err(|e| {
                tracing::debug!("Failed to create decoding key: {e}");
                AuthError::Invalid("failed to create decoding key")
            })?;

        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.validate_nbf = true;
        validation.leeway = self.config.clock_skew_seconds;
        validation.algorithms = self.config.algorithms.clone();
        validation.set_required_spec_claims(&["exp", "sub", "iss", "aud"]);

        let token_data =
            decode::<Claims>(token, &decoding_key, &validation).map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    tracing::debug!("JWT expired");
                    AuthError::Expired
                }
                _ => {
                    tracing::debug!("JWT validation failed: {e}");
                    AuthError::Invalid("token validation failed")
                }
            })?;

        let sub = token_data.claims.sub;
        if sub.trim().is_empty() {
            return Err(AuthError::Invalid("sub claim is empty"));
        }

        let jti = token_data.claims.jti;
        Ok(Subject(sub, jti))
    }
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    #[serde(default)]
    jti: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    Missing,
    Expired,
    Invalid(&'static str),
}

impl AuthError {
    fn error_code(&self) -> &'static str {
        match self {
            AuthError::Missing => "jwt_missing",
            AuthError::Expired => "jwt_expired",
            AuthError::Invalid(_) => "jwt_invalid",
        }
    }

    fn message(&self) -> String {
        match self {
            AuthError::Missing => "missing or invalid Authorization header".to_string(),
            AuthError::Expired => "token has expired".to_string(),
            AuthError::Invalid(reason) => reason.to_string(),
        }
    }
}

/// Axum middleware: verify JWT and inject Subject into request extensions.
pub async fn require_jwt_auth(
    axum::extract::State(jwt_state): axum::extract::State<JwtState>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return auth_error_response(AuthError::Missing);
        }
    };

    match jwt_state.verify_token(token).await {
        Ok(subject) => {
            request.extensions_mut().insert(subject);
            next.run(request).await
        }
        Err(err) => auth_error_response(err),
    }
}

fn auth_error_response(err: AuthError) -> Response {
    let body = json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": {
            "code": -32001,
            "message": err.message(),
            "data": {
                "type": err.error_code(),
            }
        }
    });

    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_debug_shows_prefix_when_longer_than_eight_chars() {
        let subject = Subject("user-12345678-abcdefgh".to_string(), None);
        let debug = format!("{:?}", subject);
        assert_eq!(debug, "Subject(user-123...)");
    }

    #[test]
    fn subject_debug_hides_chars_beyond_eight_when_longer_than_eight_chars() {
        let subject = Subject("user-12345678-abcdefgh".to_string(), None);
        let debug = format!("{:?}", subject);
        assert!(!debug.contains("abcdefgh"));
    }

    #[test]
    fn subject_display_redacts_long_values() {
        let subject = Subject("user-12345678-abcdefgh".to_string(), None);
        let display = format!("{}", subject);
        assert_eq!(display, "user-123...");
    }

    #[test]
    fn subject_display_shows_short_values_fully() {
        let subject = Subject("alice".to_string(), None);
        let display = format!("{}", subject);
        assert_eq!(display, "alice");
    }

    #[test]
    fn subject_display_truncates_multibyte_utf8_at_eight_chars() {
        let subject = Subject(
            "\u{1F600}\u{1F601}\u{1F602}\u{1F603}\u{1F604}\u{1F605}\u{1F606}\u{1F607}\u{1F608}"
                .to_string(),
            None,
        );
        let display = format!("{}", subject);
        assert_eq!(
            display,
            "\u{1F600}\u{1F601}\u{1F602}\u{1F603}\u{1F604}\u{1F605}\u{1F606}\u{1F607}..."
        );
    }
}
