pub mod accessibility;
pub mod builder;
pub mod error;
pub mod executor;
pub mod launcher;
pub mod schema;
pub mod session;
pub mod steps;

pub use error::BrowserError;
pub use executor::BrowserExecutor;
pub use schema::BrowserOperationTemplate;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

/// Prepared browser command data, ready for execution.
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct PreparedBrowserCommand {
    pub session_id: Option<String>,
    pub headless: bool,
    pub timeout_ms: u64,
    pub on_failure_screenshot: bool,
    pub steps: Vec<schema::BrowserStep>,
}
