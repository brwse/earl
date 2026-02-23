#![cfg(feature = "secrets-1password")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::onepassword::OpResolver;

#[test]
fn op_resolver_scheme() {
    let resolver = OpResolver::new();
    assert_eq!(resolver.scheme(), "op");
}

#[test]
fn op_resolver_parses_reference() {
    // Remove env vars so the resolver always reports missing credentials.
    // SAFETY: This test is single-threaded and no other threads read these env vars.
    unsafe {
        std::env::remove_var("OP_CONNECT_TOKEN");
        std::env::remove_var("OP_CONNECT_HOST");
    }

    let resolver = OpResolver::new();
    let err = resolver.resolve("op://vault/item/field").unwrap_err();
    assert!(
        err.to_string().contains("OP_CONNECT_TOKEN"),
        "error should mention required env vars: {}",
        err
    );
}

#[test]
fn op_resolver_rejects_invalid_reference() {
    let resolver = OpResolver::new();
    let err = resolver.resolve("op://").unwrap_err();
    assert!(err.to_string().contains("invalid"), "got: {}", err);
}
