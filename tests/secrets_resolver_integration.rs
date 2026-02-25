mod common;

use earl::secrets::resolver::SecretResolver;
use earl::secrets::store::{InMemorySecretStore, SecretStore, require_secret};
use secrecy::SecretString;

struct MockResolver {
    scheme: String,
    secrets: std::collections::HashMap<String, String>,
}

impl MockResolver {
    fn new(scheme: &str) -> Self {
        Self {
            scheme: scheme.to_string(),
            secrets: std::collections::HashMap::new(),
        }
    }

    fn with_secret(mut self, reference: &str, value: &str) -> Self {
        self.secrets
            .insert(reference.to_string(), value.to_string());
        self
    }
}

impl SecretResolver for MockResolver {
    fn scheme(&self) -> &str {
        &self.scheme
    }

    fn resolve(&self, reference: &str) -> anyhow::Result<SecretString> {
        self.secrets
            .get(reference)
            .map(|v| SecretString::new(v.clone().into()))
            .ok_or_else(|| anyhow::anyhow!("mock: secret not found: {reference}"))
    }
}

#[test]
fn local_secret_resolved_from_in_memory_store() {
    let store = InMemorySecretStore::default();
    store
        .set_secret("local.key", SecretString::new("local-value".into()))
        .unwrap();

    let resolvers: Vec<Box<dyn SecretResolver>> = vec![];

    let value = require_secret(&store, &resolvers, "local.key").unwrap();
    assert_eq!(value, "local-value");
}

#[test]
fn external_secret_resolved_via_scheme_resolver() {
    let store = InMemorySecretStore::default();
    let mock = MockResolver::new("mock").with_secret("mock://vault/item/field", "external-value");
    let resolvers: Vec<Box<dyn SecretResolver>> = vec![Box::new(mock)];

    let value = require_secret(&store, &resolvers, "mock://vault/item/field").unwrap();
    assert_eq!(value, "external-value");
}

#[test]
fn alpha_scheme_dispatched_to_alpha_resolver() {
    let store = InMemorySecretStore::default();

    let resolver_a = MockResolver::new("alpha").with_secret("alpha://secret1", "value-a");
    let resolver_b = MockResolver::new("beta").with_secret("beta://secret2", "value-b");

    let resolvers: Vec<Box<dyn SecretResolver>> = vec![Box::new(resolver_a), Box::new(resolver_b)];

    assert_eq!(
        require_secret(&store, &resolvers, "alpha://secret1").unwrap(),
        "value-a"
    );
}

#[test]
fn beta_scheme_dispatched_to_beta_resolver() {
    let store = InMemorySecretStore::default();

    let resolver_a = MockResolver::new("alpha").with_secret("alpha://secret1", "value-a");
    let resolver_b = MockResolver::new("beta").with_secret("beta://secret2", "value-b");

    let resolvers: Vec<Box<dyn SecretResolver>> = vec![Box::new(resolver_a), Box::new(resolver_b)];

    assert_eq!(
        require_secret(&store, &resolvers, "beta://secret2").unwrap(),
        "value-b"
    );
}
