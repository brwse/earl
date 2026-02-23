use std::future::Future;
use std::net::IpAddr;

use anyhow::{Context, Result, anyhow, bail};
use prost_reflect::DescriptorPool;
use serde_json::{Value, json};
use url::Url;

use crate::grpc::client::{
    DynamicRequest, DynamicResponse, dynamic_call, dynamic_call_with_reflection,
};

use earl_core::allowlist::ensure_url_allowed;
use earl_core::{ExecutionContext, PreparedBody, RawExecutionResult};

use crate::PreparedGrpcData;

/// Shared helper: validates the URL, resolves DNS, connects a tonic channel,
/// and performs the dynamic gRPC call.  Returns the RPC URL and the raw
/// `DynamicResponse` so callers can decide how to consume the result.
async fn grpc_connect_and_call<F, Fut>(
    grpc_data: &PreparedGrpcData,
    ctx: &ExecutionContext,
    host_validator: &mut F,
) -> Result<(Url, DynamicResponse)>
where
    F: FnMut(Url) -> Fut,
    Fut: Future<Output = Result<Vec<IpAddr>>>,
{
    let rpc_url = grpc_method_url(&grpc_data.url, &grpc_data.service, &grpc_data.method)?;

    ensure_url_allowed(&rpc_url, &ctx.allow_rules)?;
    let resolved_ips = host_validator(rpc_url.clone()).await?;
    if resolved_ips.is_empty() {
        bail!("host validation returned no resolved IP addresses");
    }
    if ctx.transport.proxy_url.is_some() {
        bail!("gRPC transport proxy_profile is not supported");
    }
    if ctx.transport.tls_min_version.is_some() {
        bail!("gRPC transport.tls.min_version is not supported");
    }

    // Pin the endpoint to a resolved IP to prevent TOCTOU where DNS re-resolves
    // to a different (potentially internal) address after validation.
    let resolved_ip = resolved_ips[0];
    let mut pinned_url = grpc_data.url.clone();
    pinned_url
        .set_host(Some(&resolved_ip.to_string()))
        .map_err(|_| anyhow!("failed pinning gRPC endpoint to resolved IP {resolved_ip}"))?;

    let endpoint = tonic::transport::Endpoint::new(pinned_url.to_string())
        .context("invalid gRPC endpoint URL")?
        .timeout(ctx.transport.timeout)
        .connect_timeout(ctx.transport.timeout);

    let channel = endpoint
        .connect()
        .await
        .with_context(|| format!("failed connecting to gRPC endpoint `{}`", grpc_data.url))?;

    let dynamic_request = DynamicRequest {
        body: grpc_request_body(&grpc_data.body)?,
        headers: grpc_data.headers.clone(),
        service: grpc_data.service.clone(),
        method: grpc_data.method.clone(),
    };

    let dynamic_response = if let Some(descriptor_set) = &grpc_data.descriptor_set {
        let pool = DescriptorPool::decode(descriptor_set.as_slice())
            .context("invalid gRPC descriptor set bytes")?;
        dynamic_call(channel, &pool, dynamic_request)
            .await
            .map_err(|err| anyhow!("gRPC request failed: {err}"))?
    } else {
        dynamic_call_with_reflection(channel, dynamic_request)
            .await
            .map_err(|err| anyhow!("gRPC reflection request failed: {err}"))?
    };

    Ok((rpc_url, dynamic_response))
}

/// Execute a single gRPC request and return the result.
pub async fn execute_grpc_once_with_host_validator<F, Fut>(
    grpc_data: &PreparedGrpcData,
    ctx: &ExecutionContext,
    host_validator: &mut F,
) -> Result<RawExecutionResult>
where
    F: FnMut(Url) -> Fut,
    Fut: Future<Output = Result<Vec<IpAddr>>>,
{
    let (rpc_url, dynamic_response) = grpc_connect_and_call(grpc_data, ctx, host_validator).await?;

    let (status, payload) = normalize_dynamic_response(dynamic_response);
    let payload_bytes = serde_json::to_vec(&payload).context("failed serializing gRPC payload")?;
    if payload_bytes.len() > ctx.transport.max_response_bytes {
        bail!(
            "gRPC response exceeded configured max_response_bytes ({} bytes)",
            ctx.transport.max_response_bytes
        );
    }

    Ok(RawExecutionResult {
        status,
        url: rpc_url.to_string(),
        body: payload_bytes,
        content_type: Some("application/json".to_string()),
    })
}

use earl_core::ProtocolExecutor;

/// gRPC protocol executor.
///
/// Holds a host validator closure used for DNS resolution and SSRF protection.
pub struct GrpcExecutor<F> {
    pub host_validator: F,
}

impl<F, Fut> ProtocolExecutor for GrpcExecutor<F>
where
    F: FnMut(Url) -> Fut + Send,
    Fut: Future<Output = Result<Vec<IpAddr>>> + Send,
{
    type PreparedData = PreparedGrpcData;

    async fn execute(
        &mut self,
        data: &PreparedGrpcData,
        ctx: &ExecutionContext,
    ) -> Result<RawExecutionResult> {
        execute_grpc_once_with_host_validator(data, ctx, &mut self.host_validator).await
    }
}

use earl_core::{StreamChunk, StreamMeta, StreamingProtocolExecutor};
use tokio::sync::mpsc;

/// Streaming gRPC protocol executor.
///
/// Sends each gRPC response message as an individual [`StreamChunk`] instead
/// of buffering all messages into a single JSON array.  For unary responses
/// the single message is sent as one chunk.
pub struct GrpcStreamExecutor<F> {
    pub host_validator: F,
}

impl<F, Fut> StreamingProtocolExecutor for GrpcStreamExecutor<F>
where
    F: FnMut(Url) -> Fut + Send,
    Fut: Future<Output = Result<Vec<IpAddr>>> + Send,
{
    type PreparedData = PreparedGrpcData;

    async fn execute_stream(
        &mut self,
        data: &PreparedGrpcData,
        ctx: &ExecutionContext,
        sender: mpsc::Sender<StreamChunk>,
    ) -> Result<StreamMeta> {
        let (rpc_url, dynamic_response) =
            grpc_connect_and_call(data, ctx, &mut self.host_validator).await?;

        let content_type = Some("application/json".to_string());

        let status = match dynamic_response {
            DynamicResponse::Unary(Ok(value)) => {
                let bytes =
                    serde_json::to_vec(&value).context("failed serializing gRPC unary payload")?;
                let _ = sender
                    .send(StreamChunk {
                        data: bytes,
                        content_type: content_type.clone(),
                    })
                    .await;
                0
            }
            DynamicResponse::Unary(Err(status)) => {
                let code = grpc_code(status.code());
                let payload = grpc_status_payload(&status);
                let bytes = serde_json::to_vec(&payload)
                    .context("failed serializing gRPC error payload")?;
                let _ = sender
                    .send(StreamChunk {
                        data: bytes,
                        content_type: content_type.clone(),
                    })
                    .await;
                code
            }
            DynamicResponse::Streaming(Ok(values)) => {
                let mut first_error_code = 0_u16;
                for value in values {
                    let (chunk_bytes, err_code) = match value {
                        Ok(item) => {
                            let bytes = serde_json::to_vec(&item)
                                .context("failed serializing gRPC stream message")?;
                            (bytes, 0)
                        }
                        Err(err) => {
                            let code = grpc_code(err.code());
                            let payload = grpc_status_payload(&err);
                            let bytes = serde_json::to_vec(&payload)
                                .context("failed serializing gRPC stream error")?;
                            (bytes, code)
                        }
                    };
                    if err_code != 0 && first_error_code == 0 {
                        first_error_code = err_code;
                    }
                    if sender
                        .send(StreamChunk {
                            data: chunk_bytes,
                            content_type: content_type.clone(),
                        })
                        .await
                        .is_err()
                    {
                        // Receiver dropped — stop streaming gracefully.
                        break;
                    }
                }
                first_error_code
            }
            DynamicResponse::Streaming(Err(status)) => {
                let code = grpc_code(status.code());
                let payload = grpc_status_payload(&status);
                let bytes = serde_json::to_vec(&payload)
                    .context("failed serializing gRPC stream error payload")?;
                let _ = sender
                    .send(StreamChunk {
                        data: bytes,
                        content_type: content_type.clone(),
                    })
                    .await;
                code
            }
        };

        Ok(StreamMeta {
            status,
            url: rpc_url.to_string(),
        })
    }
}

fn grpc_method_url(base: &Url, service: &str, method: &str) -> Result<Url> {
    let service = service.trim();
    let method = method.trim();
    if service.is_empty() || method.is_empty() {
        bail!("gRPC service and method must not be empty");
    }

    let mut rpc_url = base.clone();
    let prefix = rpc_url.path().trim_end_matches('/');
    let path = if prefix.is_empty() || prefix == "/" {
        format!("/{service}/{method}")
    } else {
        format!("{prefix}/{service}/{method}")
    };
    rpc_url.set_path(&path);
    Ok(rpc_url)
}

fn grpc_request_body(body: &PreparedBody) -> Result<Value> {
    match body {
        PreparedBody::Json(value) => Ok(value.clone()),
        PreparedBody::Empty => Ok(Value::Object(Default::default())),
        _ => bail!("gRPC request body must be JSON"),
    }
}

fn normalize_dynamic_response(response: DynamicResponse) -> (u16, Value) {
    match response {
        DynamicResponse::Unary(Ok(value)) => (0, value),
        DynamicResponse::Unary(Err(status)) => {
            (grpc_code(status.code()), grpc_status_payload(&status))
        }
        DynamicResponse::Streaming(Ok(values)) => {
            let mut status = 0_u16;
            let mut payload = Vec::with_capacity(values.len());
            for value in values {
                match value {
                    Ok(item) => payload.push(item),
                    Err(err) => {
                        if status == 0 {
                            status = grpc_code(err.code());
                        }
                        payload.push(grpc_status_payload(&err));
                    }
                }
            }
            (status, Value::Array(payload))
        }
        DynamicResponse::Streaming(Err(status)) => {
            (grpc_code(status.code()), grpc_status_payload(&status))
        }
    }
}

fn grpc_status_payload(status: &tonic::Status) -> Value {
    json!({
        "grpc_code": status.code().to_string(),
        "grpc_code_number": grpc_code(status.code()),
        "message": status.message(),
    })
}

fn grpc_code(code: tonic::Code) -> u16 {
    code as i32 as u16
}
