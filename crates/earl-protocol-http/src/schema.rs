use std::collections::BTreeMap;

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use earl_core::schema::{AuthTemplate, BodyTemplate, TransportTemplate};
use earl_core::with::AsJson;

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpOperationTemplate {
    pub method: String,
    pub url: String,
    pub path: Option<String>,
    #[rkyv(with = AsJson)]
    pub query: Option<BTreeMap<String, Value>>,
    #[rkyv(with = AsJson)]
    pub headers: Option<BTreeMap<String, Value>>,
    #[rkyv(with = AsJson)]
    pub cookies: Option<BTreeMap<String, Value>>,
    pub auth: Option<AuthTemplate>,
    pub body: Option<BodyTemplate>,
    #[serde(default)]
    pub stream: bool,
    pub transport: Option<TransportTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlOperationTemplate {
    #[serde(default)]
    pub method: String,
    pub url: String,
    pub path: Option<String>,
    #[rkyv(with = AsJson)]
    pub query: Option<BTreeMap<String, Value>>,
    #[rkyv(with = AsJson)]
    pub headers: Option<BTreeMap<String, Value>>,
    #[rkyv(with = AsJson)]
    pub cookies: Option<BTreeMap<String, Value>>,
    pub auth: Option<AuthTemplate>,
    pub graphql: GraphqlTemplate,
    #[serde(default)]
    pub stream: bool,
    pub transport: Option<TransportTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Archive, RkyvSerialize, RkyvDeserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlTemplate {
    pub query: String,
    pub operation_name: Option<String>,
    #[rkyv(with = AsJson)]
    pub variables: Option<Value>,
}
