use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use prost_reflect::{DescriptorPool, MethodDescriptor};
use std::str::FromStr;
use tonic::metadata::{MetadataKey, MetadataValue};
use tonic::transport::Channel;

use super::codec::JsonCodec;
use super::reflection;

/// A request object for a dynamic gRPC call.
#[derive(Debug, Clone)]
pub struct DynamicRequest {
    pub body: serde_json::Value,
    pub headers: Vec<(String, String)>,
    pub service: String,
    pub method: String,
}

/// The result of a dynamic gRPC call.
#[derive(Debug, Clone)]
pub enum DynamicResponse {
    Unary(Result<serde_json::Value, tonic::Status>),
    Streaming(Result<Vec<Result<serde_json::Value, tonic::Status>>, tonic::Status>),
}

/// Execute a dynamic gRPC call using a pre-loaded `DescriptorPool`.
pub async fn dynamic_call(
    channel: Channel,
    pool: &DescriptorPool,
    request: DynamicRequest,
) -> Result<DynamicResponse> {
    let method = pool
        .get_service_by_name(&request.service)
        .with_context(|| format!("service '{}' not found in descriptor pool", request.service))?
        .methods()
        .find(|m| m.name() == request.method)
        .with_context(|| format!("method '{}' not found in service", request.method))?;

    let mut grpc = tonic::client::Grpc::new(channel);
    grpc.ready().await.context("gRPC client not ready")?;

    match (method.is_client_streaming(), method.is_server_streaming()) {
        (false, false) => {
            let result = do_unary(&mut grpc, method, request.body, request.headers).await?;
            Ok(DynamicResponse::Unary(result))
        }
        (false, true) => {
            let result =
                do_server_streaming(&mut grpc, method, request.body, request.headers).await?;
            Ok(result)
        }
        (true, false) => {
            let items = json_array(request.body)?;
            let result = do_client_streaming(&mut grpc, method, items, request.headers).await?;
            Ok(DynamicResponse::Unary(result))
        }
        (true, true) => {
            let items = json_array(request.body)?;
            let result = do_bidi_streaming(&mut grpc, method, items, request.headers).await?;
            Ok(result)
        }
    }
}

/// Execute a dynamic gRPC call using server reflection for schema discovery.
pub async fn dynamic_call_with_reflection(
    channel: Channel,
    request: DynamicRequest,
) -> Result<DynamicResponse> {
    let fd_set = reflection::resolve_file_descriptor_set(channel.clone(), &request.service)
        .await
        .context("reflection resolution failed")?;

    let pool = DescriptorPool::from_file_descriptor_set(fd_set)
        .context("failed to build descriptor pool from reflection response")?;

    dynamic_call(channel, &pool, request).await
}

async fn do_unary(
    grpc: &mut tonic::client::Grpc<Channel>,
    method: MethodDescriptor,
    payload: serde_json::Value,
    headers: Vec<(String, String)>,
) -> Result<Result<serde_json::Value, tonic::Status>> {
    let codec = JsonCodec::new(method.input(), method.output());
    let path = http_path(&method);
    let request = build_request(payload, headers)?;

    match grpc.unary(request, path, codec).await {
        Ok(response) => Ok(Ok(response.into_inner())),
        Err(status) => Ok(Err(status)),
    }
}

async fn do_server_streaming(
    grpc: &mut tonic::client::Grpc<Channel>,
    method: MethodDescriptor,
    payload: serde_json::Value,
    headers: Vec<(String, String)>,
) -> Result<DynamicResponse> {
    let codec = JsonCodec::new(method.input(), method.output());
    let path = http_path(&method);
    let request = build_request(payload, headers)?;

    match grpc.server_streaming(request, path, codec).await {
        Ok(response) => {
            let items: Vec<_> = response.into_inner().collect().await;
            Ok(DynamicResponse::Streaming(Ok(items)))
        }
        Err(status) => Ok(DynamicResponse::Streaming(Err(status))),
    }
}

async fn do_client_streaming(
    grpc: &mut tonic::client::Grpc<Channel>,
    method: MethodDescriptor,
    items: Vec<serde_json::Value>,
    headers: Vec<(String, String)>,
) -> Result<Result<serde_json::Value, tonic::Status>> {
    let codec = JsonCodec::new(method.input(), method.output());
    let path = http_path(&method);
    let request = build_request(tokio_stream::iter(items), headers)?;

    match grpc.client_streaming(request, path, codec).await {
        Ok(response) => Ok(Ok(response.into_inner())),
        Err(status) => Ok(Err(status)),
    }
}

async fn do_bidi_streaming(
    grpc: &mut tonic::client::Grpc<Channel>,
    method: MethodDescriptor,
    items: Vec<serde_json::Value>,
    headers: Vec<(String, String)>,
) -> Result<DynamicResponse> {
    let codec = JsonCodec::new(method.input(), method.output());
    let path = http_path(&method);
    let request = build_request(tokio_stream::iter(items), headers)?;

    match grpc.streaming(request, path, codec).await {
        Ok(response) => {
            let items: Vec<_> = response.into_inner().collect().await;
            Ok(DynamicResponse::Streaming(Ok(items)))
        }
        Err(status) => Ok(DynamicResponse::Streaming(Err(status))),
    }
}

fn http_path(method: &MethodDescriptor) -> http::uri::PathAndQuery {
    let path = format!("/{}/{}", method.parent_service().full_name(), method.name());
    http::uri::PathAndQuery::from_str(&path).expect("valid gRPC path")
}

fn build_request<T>(payload: T, headers: Vec<(String, String)>) -> Result<tonic::Request<T>> {
    let mut request = tonic::Request::new(payload);
    for (k, v) in headers {
        let key = MetadataKey::from_str(&k)
            .with_context(|| format!("invalid gRPC metadata key '{k}'"))?;
        let val = MetadataValue::from_str(&v)
            .with_context(|| format!("invalid gRPC metadata value for key '{k}'"))?;
        request.metadata_mut().insert(key, val);
    }
    Ok(request)
}

fn json_array(value: serde_json::Value) -> Result<Vec<serde_json::Value>> {
    match value {
        serde_json::Value::Array(items) => Ok(items),
        _ => bail!("client streaming requires a JSON array body"),
    }
}
