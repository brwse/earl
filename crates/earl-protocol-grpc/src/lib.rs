pub mod builder;
pub mod executor;
pub mod grpc;
pub mod schema;

pub use executor::GrpcExecutor;
pub use schema::{GrpcOperationTemplate, GrpcTemplate};

/// Prepared gRPC request data, ready for execution.
#[derive(Debug, Clone)]
pub struct PreparedGrpcData {
    pub url: url::Url,
    pub headers: Vec<(String, String)>,
    pub body: earl_core::PreparedBody,
    pub service: String,
    pub method: String,
    pub descriptor_set: Option<Vec<u8>>,
}
