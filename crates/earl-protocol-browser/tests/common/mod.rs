// tests/common/mod.rs

#![allow(dead_code, unused_imports)]

pub mod server;
pub use server::*;

use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::{Mutex, OwnedMutexGuard};

use earl_core::schema::ResultTemplate;
use earl_core::transport::ResolvedTransport;
use earl_core::{CommandMode, ExecutionContext, ProtocolExecutor, Redactor};
use earl_protocol_browser::{BrowserExecutor, PreparedBrowserCommand};
use serde_json::{Map, Value};

/// A guard that holds both the process-level mutex guard and the file-based
/// advisory lock. Both are released when this guard is dropped.
pub struct ChromeGuard {
    _mutex_guard: OwnedMutexGuard<()>,
    _file: std::fs::File,
}

impl Drop for ChromeGuard {
    fn drop(&mut self) {
        use fs4::fs_std::FileExt;
        // Best-effort unlock; ignore errors during drop.
        let _ = self._file.unlock();
    }
}

/// Returns a `ChromeGuard` that serializes Chrome tests both within a single
/// test binary (via a process-level `Mutex`) and across test binaries (via a
/// file-based advisory lock using `fs4`).
pub async fn chrome_lock() -> ChromeGuard {
    // Process-level mutex — one per binary, shared across all tests in this process.
    static PROCESS_MUTEX: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    let mutex = PROCESS_MUTEX
        .get_or_init(|| Arc::new(Mutex::new(())))
        .clone();
    let mutex_guard = mutex.lock_owned().await;

    // File-based advisory lock — serializes across separate test binaries.
    let lock_path = std::env::temp_dir().join("earl_browser_tests.lock");
    let file = tokio::task::spawn_blocking(move || -> std::io::Result<std::fs::File> {
        use fs4::fs_std::FileExt;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        f.lock_exclusive()?;
        Ok(f)
    })
    .await
    .expect("spawn_blocking panicked")
    .expect("failed to acquire file lock for Chrome tests");

    ChromeGuard {
        _mutex_guard: mutex_guard,
        _file: file,
    }
}

pub fn skip_if_no_chrome() -> bool {
    if earl_protocol_browser::launcher::find_chrome().is_err() {
        eprintln!("skipping — Chrome not found on this host");
        return true;
    }
    false
}

pub fn make_context() -> ExecutionContext {
    ExecutionContext {
        key: "test".to_string(),
        mode: CommandMode::Read,
        allow_rules: vec![],
        transport: ResolvedTransport {
            timeout: Duration::from_secs(30),
            follow_redirects: true,
            max_redirect_hops: 10,
            retry_max_attempts: 0,
            retry_backoff: Duration::from_millis(100),
            retry_on_status: vec![],
            compression: false,
            tls_min_version: None,
            proxy_url: None,
            max_response_bytes: 10_000_000,
        },
        result_template: ResultTemplate::default(),
        args: Map::new(),
        redactor: Redactor::new(vec![]),
    }
}

/// Run a `PreparedBrowserCommand` through `BrowserExecutor` and parse the body as JSON.
pub async fn execute(data: PreparedBrowserCommand) -> anyhow::Result<Value> {
    let mut executor = BrowserExecutor;
    let result = executor.execute(&data, &make_context()).await?;
    Ok(serde_json::from_slice(&result.body)?)
}
