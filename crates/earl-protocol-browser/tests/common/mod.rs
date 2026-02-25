// tests/common/mod.rs

#![allow(dead_code, unused_imports)]

pub mod server;
pub use server::*;

use std::sync::Mutex;
use std::time::Duration;

use earl_core::{CommandMode, ExecutionContext, ProtocolExecutor, Redactor};
use earl_core::schema::ResultTemplate;
use earl_core::transport::ResolvedTransport;
use earl_protocol_browser::{BrowserExecutor, PreparedBrowserCommand};
use serde_json::{Map, Value};

pub static CHROME_SERIAL: Mutex<()> = Mutex::new(());

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
