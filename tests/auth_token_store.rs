mod common;

use chrono::{Duration, Utc};
use earl::auth::token_store::{OAuthTokenStore, StoredOAuthToken};
use secrecy::SecretString;

fn token(expires_at: Option<chrono::DateTime<Utc>>) -> StoredOAuthToken {
    StoredOAuthToken {
        access_token: "access-1".to_string(),
        refresh_token: Some("refresh-1".to_string()),
        token_type: Some("Bearer".to_string()),
        expires_at,
        scopes: vec!["repo".to_string()],
    }
}

#[test]
fn access_token_preserved_on_load() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let store = OAuthTokenStore::new(&secrets);

    store
        .save("github", &token(Some(Utc::now() + Duration::hours(1))))
        .unwrap();

    let loaded = store.load("github").unwrap().unwrap();
    assert_eq!(loaded.access_token, "access-1");
}

#[test]
fn refresh_token_preserved_on_load() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let store = OAuthTokenStore::new(&secrets);

    store
        .save("github", &token(Some(Utc::now() + Duration::hours(1))))
        .unwrap();

    let loaded = store.load("github").unwrap().unwrap();
    assert_eq!(loaded.refresh_token.as_deref(), Some("refresh-1"));
}

#[test]
fn delete_existing_token_returns_true() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let store = OAuthTokenStore::new(&secrets);

    store
        .save("github", &token(Some(Utc::now() + Duration::hours(1))))
        .unwrap();

    assert!(store.delete("github").unwrap());
}

#[test]
fn deleted_token_cannot_be_loaded() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    let store = OAuthTokenStore::new(&secrets);

    store
        .save("github", &token(Some(Utc::now() + Duration::hours(1))))
        .unwrap();

    store.delete("github").unwrap();
    assert!(store.load("github").unwrap().is_none());
}

#[test]
fn token_store_reports_corrupted_payload() {
    let ws = common::temp_workspace();
    let secrets =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    secrets
        .set(
            "oauth2.github.token",
            SecretString::new("not-json".to_string().into()),
        )
        .unwrap();

    let store = OAuthTokenStore::new(&secrets);
    let err = store.load("github").unwrap_err();
    assert!(err.downcast_ref::<serde_json::Error>().is_some());
}

#[test]
fn past_token_is_expired() {
    let expired = token(Some(Utc::now() - Duration::seconds(1)));
    assert!(expired.is_expired());
}

#[test]
fn token_within_safety_window_is_expired() {
    let near_expiry = token(Some(Utc::now() + Duration::seconds(10)));
    assert!(near_expiry.is_expired());
}

#[test]
fn token_outside_safety_window_is_not_expired() {
    let valid = token(Some(Utc::now() + Duration::minutes(5)));
    assert!(!valid.is_expired());
}
