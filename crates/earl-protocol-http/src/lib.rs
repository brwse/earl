pub mod builder;
pub mod executor;
pub mod schema;

pub use executor::{HttpExecutor, HttpStreamExecutor};
pub use schema::{GraphqlOperationTemplate, GraphqlTemplate, HttpOperationTemplate};

/// Prepared HTTP/GraphQL request data, ready for execution.
#[derive(Debug, Clone)]
pub struct PreparedHttpData {
    pub method: reqwest::Method,
    pub url: url::Url,
    pub query: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub cookies: Vec<(String, String)>,
    pub body: earl_core::PreparedBody,
}
