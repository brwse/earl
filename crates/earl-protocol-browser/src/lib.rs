pub mod builder;
pub mod error;
pub mod executor;
pub mod schema;
pub mod session;
pub mod steps;

pub use error::BrowserError;
pub use executor::BrowserExecutor;
pub use schema::BrowserOperationTemplate;

/// Prepared browser command data, ready for execution.
#[derive(Debug, Clone)]
pub struct PreparedBrowserCommand {
    pub session_id: Option<String>,
    pub headless: bool,
    pub timeout_ms: u64,
    pub on_failure_screenshot: bool,
    pub steps: Vec<schema::BrowserStep>,
}
