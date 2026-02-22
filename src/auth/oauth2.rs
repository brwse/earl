use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use oauth2::basic::BasicClient;
use oauth2::reqwest::Client;
use oauth2::{
    AuthType, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    DeviceAuthorizationUrl, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope,
    StandardDeviceAuthorizationResponse, TokenResponse, TokenUrl,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::config::{Config, OAuthFlow};
use crate::secrets::SecretManager;

use super::profiles::{ResolvedOAuthProfile, resolve_profile};
use super::token_store::{OAuthTokenStore, StoredOAuthToken};

#[derive(Debug, Clone)]
pub struct AuthStatus {
    pub logged_in: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

pub struct OAuthManager {
    config: Config,
    secrets: SecretManager,
    http_client: Client,
    browser_opener: BrowserOpener,
    callback_waiter: CallbackWaiter,
}

pub type CallbackFuture = Pin<Box<dyn Future<Output = Result<(String, String)>> + Send>>;
pub type BrowserOpener = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;
pub type CallbackWaiter = Arc<dyn Fn(String) -> CallbackFuture + Send + Sync>;

impl OAuthManager {
    pub fn new(config: Config, secrets: SecretManager) -> Result<Self> {
        Self::with_hooks(
            config,
            secrets,
            default_browser_opener(),
            default_callback_waiter(),
        )
    }

    pub fn with_hooks(
        config: Config,
        secrets: SecretManager,
        browser_opener: BrowserOpener,
        callback_waiter: CallbackWaiter,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .redirect(oauth2::reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed creating OAuth HTTP client")?;

        Ok(Self {
            config,
            secrets,
            http_client,
            browser_opener,
            callback_waiter,
        })
    }

    pub async fn login(&self, profile_name: &str) -> Result<()> {
        let profile =
            resolve_profile(profile_name, &self.config, &self.secrets, &self.http_client).await?;

        let token = match profile.flow {
            OAuthFlow::AuthCodePkce => match self.login_auth_code_pkce(&profile).await {
                Ok(token) => token,
                Err(err) => {
                    if profile.device_authorization_url.is_some() {
                        eprintln!(
                            "auth code flow failed for `{}`; trying device flow fallback",
                            profile.name
                        );
                        tracing::debug!(
                            profile = %profile.name,
                            error = format!("{err:#}"),
                            "auth code flow error details"
                        );
                        self.login_device_code(&profile).await?
                    } else {
                        return Err(err);
                    }
                }
            },
            OAuthFlow::DeviceCode => self.login_device_code(&profile).await?,
            OAuthFlow::ClientCredentials => self.login_client_credentials(&profile).await?,
        };

        OAuthTokenStore::new(&self.secrets).save(profile_name, &token)?;
        Ok(())
    }

    pub fn status(&self, profile_name: &str) -> Result<AuthStatus> {
        let store = OAuthTokenStore::new(&self.secrets);
        let token = store.load(profile_name)?;

        match token {
            Some(token) => Ok(AuthStatus {
                logged_in: true,
                expires_at: token.expires_at,
                scopes: token.scopes,
            }),
            None => Ok(AuthStatus {
                logged_in: false,
                expires_at: None,
                scopes: Vec::new(),
            }),
        }
    }

    pub async fn refresh(&self, profile_name: &str) -> Result<()> {
        let profile =
            resolve_profile(profile_name, &self.config, &self.secrets, &self.http_client).await?;

        let current = OAuthTokenStore::new(&self.secrets)
            .load(profile_name)?
            .ok_or_else(|| anyhow::anyhow!("no existing token for profile `{profile_name}`"))?;

        let refreshed = self.refresh_token_if_possible(&profile, current).await?;
        OAuthTokenStore::new(&self.secrets).save(profile_name, &refreshed)?;
        Ok(())
    }

    pub fn logout(&self, profile_name: &str) -> Result<bool> {
        OAuthTokenStore::new(&self.secrets).delete(profile_name)
    }

    pub async fn access_token_for_profile(&self, profile_name: &str) -> Result<String> {
        let profile =
            resolve_profile(profile_name, &self.config, &self.secrets, &self.http_client).await?;
        let store = OAuthTokenStore::new(&self.secrets);

        let token = match store.load(profile_name)? {
            Some(existing) => {
                if existing.is_expired() {
                    self.refresh_token_if_possible(&profile, existing).await?
                } else {
                    existing
                }
            }
            None => {
                if matches!(profile.flow, OAuthFlow::ClientCredentials) {
                    self.login_client_credentials(&profile).await?
                } else {
                    bail!(
                        "profile `{profile_name}` is not logged in; run `earl auth login {profile_name}`"
                    )
                }
            }
        };

        store.save(profile_name, &token)?;
        Ok(token.access_token)
    }

    async fn login_auth_code_pkce(
        &self,
        profile: &ResolvedOAuthProfile,
    ) -> Result<StoredOAuthToken> {
        let redirect_url = profile
            .redirect_url
            .clone()
            .unwrap_or_else(|| "http://127.0.0.1:8976/callback".to_string());

        let auth_url = profile
            .authorization_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("profile missing authorization_url"))?;

        let mut client = BasicClient::new(ClientId::new(profile.client_id.clone()))
            .set_auth_uri(AuthUrl::new(auth_url.clone()).context("invalid authorization_url")?)
            .set_token_uri(TokenUrl::new(profile.token_url.clone()).context("invalid token_url")?)
            .set_redirect_uri(
                RedirectUrl::new(redirect_url.clone()).context("invalid redirect_url")?,
            );

        if let Some(secret) = &profile.client_secret {
            client = client.set_client_secret(ClientSecret::new(secret.clone()));
        }

        if profile.use_auth_request_body {
            client = client.set_auth_type(AuthType::RequestBody);
        }

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut auth_req = client.authorize_url(CsrfToken::new_random);
        for scope in &profile.scopes {
            auth_req = auth_req.add_scope(Scope::new(scope.clone()));
        }

        let (authorize_url, csrf_state) = auth_req.set_pkce_challenge(pkce_challenge).url();

        println!("Open this URL in your browser:\n{authorize_url}\n");
        (self.browser_opener)(authorize_url.as_str())?;

        let (code, returned_state) = (self.callback_waiter)(redirect_url.clone()).await?;
        if returned_state != *csrf_state.secret() {
            bail!("csrf state mismatch during OAuth callback");
        }

        let token = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.secret().to_string()))
            .request_async(&self.http_client)
            .await
            .context("failed exchanging authorization code")?;

        token_response_to_stored(&token, &profile.scopes)
    }

    async fn login_device_code(&self, profile: &ResolvedOAuthProfile) -> Result<StoredOAuthToken> {
        let device_url = profile
            .device_authorization_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("profile missing device_authorization_url"))?;

        let mut client = BasicClient::new(ClientId::new(profile.client_id.clone()))
            .set_token_uri(TokenUrl::new(profile.token_url.clone()).context("invalid token_url")?)
            .set_device_authorization_url(
                DeviceAuthorizationUrl::new(device_url.clone())
                    .context("invalid device_authorization_url")?,
            );

        if let Some(secret) = &profile.client_secret {
            client = client.set_client_secret(ClientSecret::new(secret.clone()));
        }

        if profile.use_auth_request_body {
            client = client.set_auth_type(AuthType::RequestBody);
        }

        let mut req = client.exchange_device_code();
        for scope in &profile.scopes {
            req = req.add_scope(Scope::new(scope.clone()));
        }

        let details: StandardDeviceAuthorizationResponse = req
            .request_async(&self.http_client)
            .await
            .context("failed to request OAuth device code")?;

        // The user code is a short-lived, one-time code the user must manually
        // enter in a browser to complete device authorization — it is not a secret.
        print_device_code_instructions(
            details.verification_uri().as_str(),
            details.user_code().secret(),
        );

        let token = client
            .exchange_device_access_token(&details)
            .request_async(&self.http_client, tokio::time::sleep, None)
            .await
            .context("failed device code token exchange")?;

        token_response_to_stored(&token, &profile.scopes)
    }

    async fn login_client_credentials(
        &self,
        profile: &ResolvedOAuthProfile,
    ) -> Result<StoredOAuthToken> {
        let mut client = BasicClient::new(ClientId::new(profile.client_id.clone()))
            .set_token_uri(TokenUrl::new(profile.token_url.clone()).context("invalid token_url")?);

        if let Some(secret) = &profile.client_secret {
            client = client.set_client_secret(ClientSecret::new(secret.clone()));
        }

        if profile.use_auth_request_body {
            client = client.set_auth_type(AuthType::RequestBody);
        }

        let mut req = client.exchange_client_credentials();
        for scope in &profile.scopes {
            req = req.add_scope(Scope::new(scope.clone()));
        }

        let token = req
            .request_async(&self.http_client)
            .await
            .context("failed client credentials token exchange")?;

        token_response_to_stored(&token, &profile.scopes)
    }

    async fn refresh_token_if_possible(
        &self,
        profile: &ResolvedOAuthProfile,
        current: StoredOAuthToken,
    ) -> Result<StoredOAuthToken> {
        if !current.is_expired() {
            return Ok(current);
        }

        if let Some(refresh_token) = &current.refresh_token {
            let mut client = BasicClient::new(ClientId::new(profile.client_id.clone()))
                .set_token_uri(
                    TokenUrl::new(profile.token_url.clone()).context("invalid token_url")?,
                );

            if let Some(secret) = &profile.client_secret {
                client = client.set_client_secret(ClientSecret::new(secret.clone()));
            }

            if profile.use_auth_request_body {
                client = client.set_auth_type(AuthType::RequestBody);
            }

            let token = client
                .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
                .request_async(&self.http_client)
                .await
                .context("failed refreshing OAuth token")?;
            return token_response_to_stored(&token, &profile.scopes);
        }

        if matches!(profile.flow, OAuthFlow::ClientCredentials) {
            return self.login_client_credentials(profile).await;
        }

        bail!(
            "token expired for profile `{}` and no refresh token is available; run `earl auth login {}`",
            profile.name,
            profile.name
        )
    }
}

fn token_response_to_stored<T>(token: &T, fallback_scopes: &[String]) -> Result<StoredOAuthToken>
where
    T: TokenResponse,
    T::TokenType: std::fmt::Debug,
{
    let expires_at = token
        .expires_in()
        .map(|dur| chrono::Duration::from_std(dur).map(|d| Utc::now() + d))
        .transpose()?;

    let scopes = token
        .scopes()
        .map(|scopes| scopes.iter().map(|scope| scope.to_string()).collect())
        .unwrap_or_else(|| fallback_scopes.to_vec());

    Ok(StoredOAuthToken {
        access_token: token.access_token().secret().to_string(),
        refresh_token: token.refresh_token().map(|r| r.secret().to_string()),
        token_type: Some(format!("{:?}", token.token_type())),
        expires_at,
        scopes,
    })
}

fn default_browser_opener() -> BrowserOpener {
    Arc::new(|url| {
        let _ = webbrowser::open(url);
        Ok(())
    })
}

fn default_callback_waiter() -> CallbackWaiter {
    Arc::new(|redirect_url| Box::pin(async move { wait_for_auth_callback(&redirect_url).await }))
}

async fn wait_for_auth_callback(redirect_url: &str) -> Result<(String, String)> {
    let parsed = Url::parse(redirect_url).context("invalid redirect URL")?;
    if parsed.scheme() != "http" {
        bail!("redirect URL `{redirect_url}` must use http scheme for local callback listener");
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("redirect URL missing host"))?
        .to_string();
    if !is_loopback_host(&host) {
        bail!("redirect URL host `{host}` must be loopback (127.0.0.1, ::1, or localhost)");
    }

    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("redirect URL missing port"))?;
    let expected_path = parsed.path().to_string();

    let listener = TcpListener::bind((host.as_str(), port))
        .await
        .with_context(|| format!("failed to bind redirect callback server on {host}:{port}"))?;

    let (mut socket, _) = listener.accept().await?;
    let mut buf = vec![0_u8; 8192];
    let read = socket.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..read]);
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty callback request"))?;

    let path = first_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("invalid callback request line"))?;

    let callback_url =
        Url::parse(&format!("http://localhost{path}")).context("failed parsing callback URL")?;
    if callback_url.path() != expected_path {
        bail!(
            "authorization callback path mismatch: expected `{expected_path}`, got `{}`",
            callback_url.path()
        );
    }

    let query_pairs: BTreeMap<String, String> = callback_url.query_pairs().into_owned().collect();

    let code = query_pairs
        .get("code")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("authorization callback missing `code`"))?;
    let state = query_pairs
        .get("state")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("authorization callback missing `state`"))?;

    let body = "Authentication completed. You can return to the terminal.";
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    socket.write_all(response.as_bytes()).await?;

    Ok((code, state))
}

/// Print device-flow instructions to the terminal.
///
/// The `user_code` is an ephemeral, one-time code that the user must enter
/// in a browser to authorise the device. It is not a long-lived secret.
fn print_device_code_instructions(verification_uri: &str, user_code: &str) {
    use std::io::Write;
    let mut out = std::io::stderr().lock();
    let _ = writeln!(out, "Open this URL in your browser:");
    let _ = writeln!(out, "{verification_uri}");
    let _ = write!(out, "and enter the code: ");
    let _ = out.write_all(user_code.as_bytes());
    let _ = writeln!(out);
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false)
}
