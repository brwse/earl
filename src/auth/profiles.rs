use anyhow::{Context, Result, bail};
use oauth2::reqwest::Client;
use serde::Deserialize;
use url::Url;

use crate::config::{Config, OAuthFlow};
use crate::secrets::SecretManager;
use crate::secrets::store::require_secret;
use crate::security::dns::resolve_and_validate_host;

#[derive(Debug, Clone)]
pub struct ResolvedOAuthProfile {
    pub name: String,
    pub flow: OAuthFlow,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: String,
    pub device_authorization_url: Option<String>,
    pub redirect_url: Option<String>,
    pub scopes: Vec<String>,
    pub use_auth_request_body: bool,
}

#[derive(Debug, Deserialize)]
struct OidcMetadata {
    authorization_endpoint: Option<String>,
    token_endpoint: Option<String>,
    device_authorization_endpoint: Option<String>,
}

pub async fn resolve_profile(
    name: &str,
    config: &Config,
    secrets: &SecretManager,
    http_client: &Client,
) -> Result<ResolvedOAuthProfile> {
    let profile = config
        .auth
        .profiles
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("unknown auth profile `{name}`"))?;

    let mut authorization_url = profile.authorization_url.clone();
    let mut token_url = profile.token_url.clone();
    let mut device_authorization_url = profile.device_authorization_url.clone();

    if let Some(issuer) = &profile.issuer {
        let metadata = discover_oidc(issuer, http_client).await?;
        if authorization_url.is_none() {
            authorization_url = metadata.authorization_endpoint;
        }
        if token_url.is_none() {
            token_url = metadata.token_endpoint;
        }
        if device_authorization_url.is_none() {
            device_authorization_url = metadata.device_authorization_endpoint;
        }
    }

    let token_url =
        token_url.ok_or_else(|| anyhow::anyhow!("profile `{name}` missing token_url"))?;

    let client_secret = match &profile.client_secret_key {
        Some(secret_key) => Some(require_secret(
            secrets.store(),
            secrets.resolvers(),
            secret_key,
        )?),
        None => None,
    };

    if matches!(profile.flow, OAuthFlow::AuthCodePkce) && authorization_url.is_none() {
        bail!("profile `{name}` requires authorization_url for auth_code_pkce");
    }

    if matches!(profile.flow, OAuthFlow::DeviceCode) && device_authorization_url.is_none() {
        bail!("profile `{name}` requires device_authorization_url for device_code");
    }

    validate_oauth_endpoint("token_url", &token_url).await?;
    if let Some(url) = authorization_url.as_ref() {
        validate_oauth_endpoint("authorization_url", url).await?;
    }
    if let Some(url) = device_authorization_url.as_ref() {
        validate_oauth_endpoint("device_authorization_url", url).await?;
    }

    Ok(ResolvedOAuthProfile {
        name: name.to_string(),
        flow: profile.flow.clone(),
        client_id: profile.client_id.clone(),
        client_secret,
        authorization_url,
        token_url,
        device_authorization_url,
        redirect_url: profile.redirect_url.clone(),
        scopes: profile.scopes.clone(),
        use_auth_request_body: profile.use_auth_request_body,
    })
}

async fn discover_oidc(issuer: &str, http_client: &Client) -> Result<OidcMetadata> {
    let parsed = Url::parse(issuer).with_context(|| format!("invalid issuer URL `{issuer}`"))?;
    validate_endpoint_url("issuer", &parsed).await?;

    let issuer = parsed.as_str().trim_end_matches('/');
    let url = format!("{issuer}/.well-known/openid-configuration");

    let response = http_client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed OIDC discovery request to `{url}`"))?;

    if !response.status().is_success() {
        bail!(
            "OIDC discovery failed at `{url}` with status {}",
            response.status()
        );
    }

    let body = response
        .text()
        .await
        .with_context(|| format!("failed reading OIDC discovery response from `{url}`"))?;
    serde_json::from_str::<OidcMetadata>(&body)
        .with_context(|| format!("invalid OIDC discovery response from `{url}`"))
}

async fn validate_oauth_endpoint(field: &str, raw: &str) -> Result<()> {
    let parsed = Url::parse(raw).with_context(|| format!("invalid {field} URL `{raw}`"))?;
    validate_endpoint_url(field, &parsed).await
}

async fn validate_endpoint_url(field: &str, parsed: &Url) -> Result<()> {
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("{field} URL `{parsed}` is missing host"))?;

    match parsed.scheme() {
        "https" => {
            resolve_and_validate_host(host, false).await?;
            Ok(())
        }
        "http" if is_loopback_host(host) => Ok(()),
        scheme => bail!(
            "{field} URL `{parsed}` uses unsupported scheme `{scheme}`; expected https (or loopback http for local development)"
        ),
    }
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}
