use std::collections::BTreeMap;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use earl_core::schema::{AuthTemplate, TransportTemplate};
use earl_core::with::AsJson;

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcOperationTemplate {
    pub url: String,
    #[rkyv(with = AsJson)]
    pub headers: Option<BTreeMap<String, Value>>,
    pub auth: Option<AuthTemplate>,
    pub grpc: GrpcTemplate,
    #[serde(default)]
    pub stream: bool,
    pub transport: Option<TransportTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcTemplate {
    pub service: String,
    pub method: String,
    #[rkyv(with = AsJson)]
    pub body: Option<Value>,
    pub descriptor_set_file: Option<String>,
}
