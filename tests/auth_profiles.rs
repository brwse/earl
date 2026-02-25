mod common;

use std::collections::BTreeMap;

use earl::auth::profiles::resolve_profile;
use earl::config::{AuthConfig, Config, OAuthFlow, OAuthProfile, SandboxConfig};
use httpmock::prelude::*;
use oauth2::reqwest::Client;
use secrecy::SecretString;

fn base_profile(flow: OAuthFlow) -> OAuthProfile {
    OAuthProfile {
        flow,
        client_id: "client-123".to_string(),
        client_secret_key: None,
        issuer: None,
        authorization_url: None,
        token_url: None,
        device_authorization_url: None,
        redirect_url: Some("http://127.0.0.1:8976/callback".to_string()),
        scopes: vec!["repo".to_string()],
        use_auth_request_body: false,
    }
}

async fn oidc_resolved_profile() -> earl::auth::profiles::ResolvedOAuthProfile {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(GET).path("/.well-known/openid-configuration");
            then.status(200).json_body_obj(&serde_json::json!({
                "authorization_endpoint": format!("{}/oauth/authorize", server.base_url()),
                "token_endpoint": format!("{}/oauth/token", server.base_url()),
                "device_authorization_endpoint": format!("{}/oauth/device", server.base_url()),
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    let mut profile = base_profile(OAuthFlow::AuthCodePkce);
    profile.issuer = Some(server.base_url());

    let mut profiles = BTreeMap::new();
    profiles.insert("github".to_string(), profile);

    let cfg = Config {
        search: Default::default(),
        auth: AuthConfig {
            profiles,
            jwt: None,
        },
        network: Default::default(),
        sandbox: SandboxConfig::default(),
        policy: vec![],
        environments: Default::default(),
    };

    let http_client = Client::builder().build().unwrap();
    resolve_profile("github", &cfg, &secrets, &http_client)
        .await
        .unwrap()
}

#[tokio::test]
async fn oidc_discovery_populates_authorization_url() {
    let resolved = oidc_resolved_profile().await;
    assert!(resolved.authorization_url.unwrap().contains("/oauth/authorize"));
}

#[tokio::test]
async fn oidc_discovery_populates_token_url() {
    let resolved = oidc_resolved_profile().await;
    assert!(resolved.token_url.contains("/oauth/token"));
}

#[tokio::test]
async fn resolves_client_secret_from_secrets() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    secrets
        .set(
            "github.oauth.client_secret",
            SecretString::new("super-secret".to_string().into()),
        )
        .unwrap();

    let mut profile = base_profile(OAuthFlow::AuthCodePkce);
    profile.authorization_url = Some("http://127.0.0.1/oauth/authorize".to_string());
    profile.token_url = Some("http://127.0.0.1/oauth/token".to_string());
    profile.client_secret_key = Some("github.oauth.client_secret".to_string());

    let mut profiles = BTreeMap::new();
    profiles.insert("github".to_string(), profile);

    let cfg = Config {
        search: Default::default(),
        auth: AuthConfig {
            profiles,
            jwt: None,
        },
        network: Default::default(),
        sandbox: SandboxConfig::default(),
        policy: vec![],
        environments: Default::default(),
    };

    let http_client = Client::builder().build().unwrap();
    let resolved = resolve_profile("github", &cfg, &secrets, &http_client)
        .await
        .unwrap();

    assert_eq!(resolved.client_secret.as_deref(), Some("super-secret"));
}

#[tokio::test]
async fn fails_when_required_auth_code_endpoint_is_missing() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    let mut profile = base_profile(OAuthFlow::AuthCodePkce);
    profile.token_url = Some("https://example.com/oauth/token".to_string());

    let mut profiles = BTreeMap::new();
    profiles.insert("github".to_string(), profile);

    let cfg = Config {
        search: Default::default(),
        auth: AuthConfig {
            profiles,
            jwt: None,
        },
        network: Default::default(),
        sandbox: SandboxConfig::default(),
        policy: vec![],
        environments: Default::default(),
    };

    let http_client = Client::builder().build().unwrap();
    assert!(
        resolve_profile("github", &cfg, &secrets, &http_client)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn fails_when_device_flow_endpoint_missing() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    let mut profile = base_profile(OAuthFlow::DeviceCode);
    profile.token_url = Some("https://issuer/token".to_string());

    let mut profiles = BTreeMap::new();
    profiles.insert("github".to_string(), profile);

    let cfg = Config {
        search: Default::default(),
        auth: AuthConfig {
            profiles,
            jwt: None,
        },
        network: Default::default(),
        sandbox: SandboxConfig::default(),
        policy: vec![],
        environments: Default::default(),
    };

    let http_client = Client::builder().build().unwrap();
    assert!(
        resolve_profile("github", &cfg, &secrets, &http_client)
            .await
            .is_err()
    );
}
