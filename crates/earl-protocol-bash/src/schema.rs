use std::collections::BTreeMap;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use earl_core::schema::TransportTemplate;
use earl_core::with::AsJson;

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct BashOperationTemplate {
    pub bash: BashScriptTemplate,
    #[serde(default)]
    pub stream: bool,
    pub transport: Option<TransportTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct BashScriptTemplate {
    pub script: String,
    #[rkyv(with = AsJson)]
    #[serde(default)]
    pub env: Option<BTreeMap<String, Value>>,
    pub cwd: Option<String>,
    pub sandbox: Option<BashSandboxTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct BashSandboxTemplate {
    pub network: Option<bool>,
    pub writable_paths: Option<Vec<String>>,
    pub max_time_ms: Option<u64>,
    pub max_output_bytes: Option<u64>,
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_time_ms: Option<u64>,
}
