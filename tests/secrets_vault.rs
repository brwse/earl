#![cfg(feature = "secrets-vault")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::vault::VaultResolver;

#[test]
fn vault_resolver_scheme() {
    let resolver = VaultResolver::new();
    assert_eq!(resolver.scheme(), "vault");
}

#[test]
fn vault_resolver_requires_env_vars() {
    // Clear env vars so the resolver always reports missing credentials.
    // SAFETY: This test is single-threaded and no other threads read these env vars.
    unsafe {
        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
    }

    let resolver = VaultResolver::new();
    let err = resolver
        .resolve("vault://secret/data/myapp#api_key")
        .unwrap_err();
    assert!(
        err.to_string().contains("VAULT_ADDR") || err.to_string().contains("VAULT_TOKEN"),
        "error should mention required env vars: {}",
        err
    );
}

#[test]
fn vault_resolver_parses_path_and_field() {
    let resolver = VaultResolver::new();
    let err = resolver.resolve("vault://").unwrap_err();
    assert!(err.to_string().contains("invalid"), "got: {}", err);
}
