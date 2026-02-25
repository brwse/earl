use std::fs;

use earl::secrets::metadata_index::{load_index, save_index};
use earl::secrets::resolver::SecretResolver;
use earl::secrets::store::{InMemorySecretStore, SecretIndex, SecretStore, require_secret};
use secrecy::SecretString;
use tempfile::tempdir;

#[test]
fn upsert_deduplicates_identical_key() {
    let mut index = SecretIndex::default();
    index.upsert("github.token");
    index.upsert("github.token");
    index.upsert("search.api_key");

    assert_eq!(index.list().len(), 2);
}

#[test]
fn upsert_makes_key_retrievable() {
    let mut index = SecretIndex::default();
    index.upsert("github.token");

    assert!(index.get("github.token").is_some());
}

#[test]
fn remove_makes_key_unretrievable() {
    let mut index = SecretIndex::default();
    index.upsert("github.token");
    index.upsert("search.api_key");
    index.remove("github.token");

    assert!(index.get("github.token").is_none());
}

#[test]
fn remove_decrements_list_length() {
    let mut index = SecretIndex::default();
    index.upsert("github.token");
    index.upsert("search.api_key");
    index.remove("github.token");

    assert_eq!(index.list().len(), 1);
}

#[test]
fn save_and_load_roundtrips_index() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("state/secrets-index.json");

    let mut index = SecretIndex::default();
    index.upsert("github.token");
    save_index(&path, &index).unwrap();

    let loaded = load_index(&path).unwrap();
    assert!(loaded.get("github.token").is_some());
}

#[test]
fn load_index_returns_error_for_corrupt_json() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("state/secrets-index.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{not-json").unwrap();

    assert!(load_index(&path).is_err());
}

#[test]
fn require_secret_returns_value_from_store() {
    let store = InMemorySecretStore::default();
    store
        .set_secret(
            "github.token",
            SecretString::new("secret-value".to_string().into()),
        )
        .unwrap();

    let resolvers: Vec<Box<dyn SecretResolver>> = vec![];

    let value = require_secret(&store, &resolvers, "github.token").unwrap();
    assert_eq!(value, "secret-value");
}

#[test]
fn require_secret_errors_for_missing_key() {
    let store = InMemorySecretStore::default();
    let resolvers: Vec<Box<dyn SecretResolver>> = vec![];

    require_secret(&store, &resolvers, "missing").unwrap_err();
}

// ── SecretResolver dispatch tests ────────────────────────────

struct MockResolver {
    scheme: String,
    value: String,
}

impl SecretResolver for MockResolver {
    fn scheme(&self) -> &str {
        &self.scheme
    }
    fn resolve(&self, _reference: &str) -> anyhow::Result<SecretString> {
        Ok(SecretString::new(self.value.clone().into()))
    }
}

#[test]
fn require_secret_dispatches_to_resolver_by_scheme() {
    let store = InMemorySecretStore::default();
    let resolver = MockResolver {
        scheme: "mock".to_string(),
        value: "resolved-value".to_string(),
    };
    let resolvers: Vec<Box<dyn SecretResolver>> = vec![Box::new(resolver)];

    let value = require_secret(&store, &resolvers, "mock://some/path").unwrap();
    assert_eq!(value, "resolved-value");
}

#[test]
fn require_secret_falls_back_to_store_for_plain_keys() {
    let store = InMemorySecretStore::default();
    store
        .set_secret(
            "github.token",
            SecretString::new("keychain-value".to_string().into()),
        )
        .unwrap();
    let resolvers: Vec<Box<dyn SecretResolver>> = vec![];

    let value = require_secret(&store, &resolvers, "github.token").unwrap();
    assert_eq!(value, "keychain-value");
}

#[test]
fn require_secret_errors_for_unknown_scheme() {
    let store = InMemorySecretStore::default();
    let resolvers: Vec<Box<dyn SecretResolver>> = vec![];

    require_secret(&store, &resolvers, "unknown://path").unwrap_err();
}
