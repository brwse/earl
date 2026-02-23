pub mod builder;
pub mod executor;
pub mod sandbox;
pub mod schema;

pub use executor::BashExecutor;
pub use executor::BashStreamExecutor;
pub use schema::{BashOperationTemplate, BashSandboxTemplate, BashScriptTemplate};

/// Resolved sandbox settings for bash execution.
#[derive(Debug, Clone)]
pub struct ResolvedBashSandbox {
    pub network: bool,
    pub writable_paths: Vec<String>,
    pub max_time_ms: Option<u64>,
    pub max_output_bytes: Option<usize>,
}

/// Prepared bash script data, ready for execution.
#[derive(Debug, Clone)]
pub struct PreparedBashScript {
    pub script: String,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    pub stdin: Option<String>,
    pub sandbox: ResolvedBashSandbox,
}
