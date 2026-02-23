#![cfg(feature = "secrets-gcp")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::gcp::GcpResolver;

#[test]
fn gcp_resolver_scheme() {
    let resolver = GcpResolver::new();
    assert_eq!(resolver.scheme(), "gcp");
}

#[test]
fn gcp_resolver_rejects_empty() {
    let resolver = GcpResolver::new();
    let err = resolver.resolve("gcp://").unwrap_err();
    assert!(err.to_string().contains("invalid"), "got: {}", err);
}

#[test]
fn gcp_resolver_rejects_missing_secret_name() {
    let resolver = GcpResolver::new();
    let err = resolver.resolve("gcp://my-project").unwrap_err();
    assert!(
        err.to_string().contains("invalid") || err.to_string().contains("expected"),
        "got: {}",
        err
    );
}
