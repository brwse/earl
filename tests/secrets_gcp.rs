#![cfg(feature = "secrets-gcp")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::gcp::GcpResolver;

#[test]
fn resolver_scheme_is_gcp() {
    let resolver = GcpResolver::new();
    assert_eq!(resolver.scheme(), "gcp");
}

#[test]
fn empty_reference_returns_error() {
    let resolver = GcpResolver::new();
    assert!(resolver.resolve("gcp://").is_err());
}

#[test]
fn missing_secret_name_returns_error() {
    let resolver = GcpResolver::new();
    assert!(resolver.resolve("gcp://my-project").is_err());
}
