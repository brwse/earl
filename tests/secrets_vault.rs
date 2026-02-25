#![cfg(feature = "secrets-vault")]

use earl::secrets::resolver::SecretResolver;
use earl::secrets::resolvers::vault::VaultResolver;

static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Removes an environment variable on construction and restores it on drop.
/// Must be used inside a block guarded by `ENV_MUTEX`.
struct EnvRestore {
    name: &'static str,
    saved: Option<String>,
}

impl EnvRestore {
    fn remove(name: &'static str) -> Self {
        let saved = std::env::var(name).ok();
        // SAFETY: guarded by ENV_MUTEX in all callers.
        unsafe { std::env::remove_var(name) };
        Self { name, saved }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        unsafe {
            match self.saved.take() {
                Some(v) => std::env::set_var(self.name, v),
                None => std::env::remove_var(self.name),
            }
        }
    }
}

#[test]
fn vault_resolver_scheme_is_vault() {
    let resolver = VaultResolver::new();
    assert_eq!(resolver.scheme(), "vault");
}

#[test]
fn missing_vault_credentials_returns_error() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: ENV_MUTEX ensures no other test in this binary concurrently
    // reads or writes VAULT_ADDR / VAULT_TOKEN.
    let _addr = EnvRestore::remove("VAULT_ADDR");
    let _token = EnvRestore::remove("VAULT_TOKEN");

    let resolver = VaultResolver::new();
    resolver.resolve("vault://secret/myapp#api_key").unwrap_err();
}

#[test]
fn empty_vault_url_returns_error() {
    let resolver = VaultResolver::new();
    resolver.resolve("vault://").unwrap_err();
}
