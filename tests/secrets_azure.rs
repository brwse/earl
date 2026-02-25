#![cfg(feature = "secrets-azure")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::azure::AzureResolver;

#[test]
fn scheme_is_az() {
    let resolver = AzureResolver::new();
    assert_eq!(resolver.scheme(), "az");
}

#[test]
fn empty_reference_returns_error() {
    let resolver = AzureResolver::new();
    assert!(resolver.resolve("az://").is_err());
}

#[test]
fn vault_without_secret_returns_error() {
    let resolver = AzureResolver::new();
    assert!(resolver.resolve("az://my-vault").is_err());
}
