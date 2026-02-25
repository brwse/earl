mod common;

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Duration, Utc};
use earl::auth::oauth2::{BrowserOpener, CallbackFuture, CallbackWaiter, OAuthManager};
use earl::auth::token_store::{OAuthTokenStore, StoredOAuthToken};
use earl::config::{AuthConfig, Config, OAuthFlow, OAuthProfile, SandboxConfig};
use earl::secrets::SecretManager;
use earl::secrets::store::{InMemorySecretStore, SecretStore};
use httpmock::prelude::*;
use secrecy::ExposeSecret;

fn make_profile(flow: OAuthFlow, base_url: &str) -> OAuthProfile {
    OAuthProfile {
        flow,
        client_id: "client-id".to_string(),
        client_secret_key: None,
        issuer: None,
        authorization_url: Some(format!("{base_url}/authorize")),
        token_url: Some(format!("{base_url}/token")),
        device_authorization_url: Some(format!("{base_url}/device")),
        redirect_url: Some("http://127.0.0.1:8976/callback".to_string()),
        scopes: vec!["repo".to_string()],
        use_auth_request_body: true,
    }
}

fn make_config(profile_name: &str, profile: OAuthProfile) -> Config {
    let mut profiles = BTreeMap::new();
    profiles.insert(profile_name.to_string(), profile);
    Config {
        search: Default::default(),
        auth: AuthConfig {
            profiles,
            jwt: None,
        },
        network: Default::default(),
        sandbox: SandboxConfig::default(),
        policy: vec![],
        environments: Default::default(),
    }
}

#[tokio::test]
async fn client_credentials_returns_access_token() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "access-cc",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "github",
        make_profile(OAuthFlow::ClientCredentials, &server.base_url()),
    );

    let oauth = OAuthManager::new(cfg, secrets).unwrap();
    let token = oauth.access_token_for_profile("github").await.unwrap();
    assert_eq!(token, "access-cc");
}

#[tokio::test]
async fn client_credentials_sets_logged_in_status() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "access-cc",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "github",
        make_profile(OAuthFlow::ClientCredentials, &server.base_url()),
    );

    let oauth = OAuthManager::new(cfg, secrets).unwrap();
    oauth.access_token_for_profile("github").await.unwrap();

    let status = oauth.status("github").unwrap();
    assert!(status.logged_in);
}

#[tokio::test]
async fn client_credentials_records_scopes_in_status() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "access-cc",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "github",
        make_profile(OAuthFlow::ClientCredentials, &server.base_url()),
    );

    let oauth = OAuthManager::new(cfg, secrets).unwrap();
    oauth.access_token_for_profile("github").await.unwrap();

    let status = oauth.status("github").unwrap();
    assert_eq!(status.scopes, vec!["repo".to_string()]);
}

#[tokio::test]
async fn refresh_flow_returns_new_access_token() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "new-access",
                "refresh_token": "new-refresh",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "github",
        make_profile(OAuthFlow::AuthCodePkce, &server.base_url()),
    );

    let store = OAuthTokenStore::new(&secrets);
    store
        .save(
            "github",
            &StoredOAuthToken {
                access_token: "old-access".to_string(),
                refresh_token: Some("old-refresh".to_string()),
                token_type: Some("Bearer".to_string()),
                expires_at: Some(Utc::now() - Duration::minutes(5)),
                scopes: vec!["repo".to_string()],
            },
        )
        .unwrap();

    let oauth = OAuthManager::new(cfg, secrets).unwrap();
    let token = oauth.access_token_for_profile("github").await.unwrap();
    assert_eq!(token, "new-access");
}

#[tokio::test]
async fn refresh_flow_persists_rotated_refresh_token() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "new-access",
                "refresh_token": "new-refresh",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let mem_store = InMemorySecretStore::default();
    let secrets = SecretManager::with_store_and_index(
        Box::new(mem_store.clone()),
        ws.root.path().join("state/secrets-index.json"),
    );
    let cfg = make_config(
        "github",
        make_profile(OAuthFlow::AuthCodePkce, &server.base_url()),
    );

    let store = OAuthTokenStore::new(&secrets);
    store
        .save(
            "github",
            &StoredOAuthToken {
                access_token: "old-access".to_string(),
                refresh_token: Some("old-refresh".to_string()),
                token_type: Some("Bearer".to_string()),
                expires_at: Some(Utc::now() - Duration::minutes(5)),
                scopes: vec!["repo".to_string()],
            },
        )
        .unwrap();

    let oauth = OAuthManager::new(cfg, secrets).unwrap();
    oauth.access_token_for_profile("github").await.unwrap();

    let raw = mem_store
        .get_secret("oauth2.github.token")
        .unwrap()
        .expect("rotated token should be persisted");
    let loaded: StoredOAuthToken = serde_json::from_str(raw.expose_secret()).unwrap();
    assert_eq!(loaded.refresh_token.as_deref(), Some("new-refresh"));
}

#[tokio::test]
async fn device_flow_login_succeeds() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/device");
            then.status(200).json_body_obj(&serde_json::json!({
                "device_code": "device-1",
                "user_code": "ABCD-EFGH",
                "verification_uri": "https://example.com/activate",
                "expires_in": 600,
                "interval": 1
            }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "device-access",
                "refresh_token": "device-refresh",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "device",
        make_profile(OAuthFlow::DeviceCode, &server.base_url()),
    );

    let oauth = OAuthManager::new(cfg, secrets).unwrap();
    oauth.login("device").await.unwrap();

    let status = oauth.status("device").unwrap();
    assert!(status.logged_in);
}

#[tokio::test]
async fn auth_code_falls_back_to_device_flow_when_callback_fails() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/device");
            then.status(200).json_body_obj(&serde_json::json!({
                "device_code": "device-2",
                "user_code": "IJKL-MNOP",
                "verification_uri": "https://example.com/activate",
                "expires_in": 600,
                "interval": 1
            }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "fallback-access",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "hybrid",
        make_profile(OAuthFlow::AuthCodePkce, &server.base_url()),
    );

    let browser_opener: BrowserOpener = Arc::new(|_| Ok(()));
    let callback_waiter: CallbackWaiter = Arc::new(|_redirect_url| {
        let fut: CallbackFuture =
            Box::pin(async { Ok(("code-123".to_string(), "wrong-state".to_string())) });
        fut
    });

    let oauth = OAuthManager::with_hooks(cfg, secrets, browser_opener, callback_waiter).unwrap();
    oauth.login("hybrid").await.unwrap();

    let status = oauth.status("hybrid").unwrap();
    assert!(status.logged_in);
}

#[tokio::test]
async fn auth_code_fallback_to_device_flow_records_scopes() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/device");
            then.status(200).json_body_obj(&serde_json::json!({
                "device_code": "device-2",
                "user_code": "IJKL-MNOP",
                "verification_uri": "https://example.com/activate",
                "expires_in": 600,
                "interval": 1
            }));
        })
        .await;
    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body_obj(&serde_json::json!({
                "access_token": "fallback-access",
                "token_type": "Bearer",
                "expires_in": 3600
            }));
        })
        .await;

    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let cfg = make_config(
        "hybrid",
        make_profile(OAuthFlow::AuthCodePkce, &server.base_url()),
    );

    let browser_opener: BrowserOpener = Arc::new(|_| Ok(()));
    let callback_waiter: CallbackWaiter = Arc::new(|_redirect_url| {
        let fut: CallbackFuture =
            Box::pin(async { Ok(("code-123".to_string(), "wrong-state".to_string())) });
        fut
    });

    let oauth = OAuthManager::with_hooks(cfg, secrets, browser_opener, callback_waiter).unwrap();
    oauth.login("hybrid").await.unwrap();

    let status = oauth.status("hybrid").unwrap();
    assert_eq!(status.scopes, vec!["repo".to_string()]);
}
