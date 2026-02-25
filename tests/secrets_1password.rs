#![cfg(feature = "secrets-1password")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::onepassword::OpResolver;

#[test]
fn op_resolver_scheme_is_op() {
    let resolver = OpResolver::new();
    assert_eq!(resolver.scheme(), "op");
}

#[test]
#[ignore = "invokes the op CLI fallback; succeeds (and therefore fails this test) when op is installed and authenticated"]
fn missing_connect_token_returns_error() {
    // SAFETY: test is #[ignore] and must be run in isolation; mutates
    // OP_CONNECT_TOKEN / OP_CONNECT_HOST env vars.
    unsafe {
        std::env::remove_var("OP_CONNECT_TOKEN");
        std::env::remove_var("OP_CONNECT_HOST");
    }

    let resolver = OpResolver::new();
    resolver.resolve("op://vault/item/field").unwrap_err();
}

#[test]
fn empty_reference_returns_error() {
    let resolver = OpResolver::new();
    resolver.resolve("op://").unwrap_err();
}
