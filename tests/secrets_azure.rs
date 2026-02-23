#![cfg(feature = "secrets-azure")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::azure::AzureResolver;

#[test]
fn azure_resolver_scheme() {
    let resolver = AzureResolver::new();
    assert_eq!(resolver.scheme(), "az");
}

#[test]
fn azure_resolver_rejects_empty() {
    let resolver = AzureResolver::new();
    let err = resolver.resolve("az://").unwrap_err();
    assert!(err.to_string().contains("invalid"), "got: {}", err);
}

#[test]
fn azure_resolver_rejects_missing_secret() {
    let resolver = AzureResolver::new();
    let err = resolver.resolve("az://my-vault").unwrap_err();
    assert!(
        err.to_string().contains("invalid") || err.to_string().contains("expected"),
        "got: {}",
        err
    );
}
