use prost::Message;
use prost_types::{FileDescriptorProto, FileDescriptorSet};
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Streaming;
use tonic::transport::Channel;
use tonic_reflection::pb::v1::{
    ServerReflectionRequest, ServerReflectionResponse,
    server_reflection_client::ServerReflectionClient, server_reflection_request::MessageRequest,
    server_reflection_response::MessageResponse,
};

#[derive(Debug, thiserror::Error)]
pub enum ReflectionError {
    #[error("Failed to start reflection stream, reflection might not be supported: '{0}'")]
    StreamInitFailed(#[source] tonic::Status),

    #[error("Reflection stream returned an error status: '{0}'")]
    StreamFailure(#[source] tonic::Status),

    #[error("Reflection stream closed unexpectedly")]
    StreamClosed,

    #[error("Failed to send request to reflection stream")]
    SendFailed,

    #[error("Server returned reflection error code {code}: {message}")]
    ServerError { code: i32, message: String },

    #[error("Failed to decode FileDescriptorProto: {0}")]
    DecodeError(#[from] prost::DecodeError),
}

const EMPTY_HOST: &str = "";

/// Resolve the complete `FileDescriptorSet` for a symbol via server reflection.
///
/// Opens a bidi stream to the reflection service, fetches the file defining `symbol`,
/// and recursively fetches all transitive dependencies.
pub async fn resolve_file_descriptor_set(
    channel: Channel,
    symbol: &str,
) -> Result<FileDescriptorSet, ReflectionError> {
    let mut client = ServerReflectionClient::new(channel);

    let (tx, rx) = mpsc::channel(100);

    let mut response_stream = client
        .server_reflection_info(ReceiverStream::new(rx))
        .await
        .map_err(ReflectionError::StreamInitFailed)?
        .into_inner();

    let req = ServerReflectionRequest {
        host: EMPTY_HOST.to_string(),
        message_request: Some(MessageRequest::FileContainingSymbol(symbol.to_string())),
    };

    tx.send(req)
        .await
        .map_err(|_| ReflectionError::SendFailed)?;

    let file_map = collect_descriptors(&mut response_stream, tx).await?;

    Ok(FileDescriptorSet {
        file: file_map.into_values().collect(),
    })
}

async fn collect_descriptors(
    response_stream: &mut Streaming<ServerReflectionResponse>,
    request_channel: mpsc::Sender<ServerReflectionRequest>,
) -> Result<HashMap<String, FileDescriptorProto>, ReflectionError> {
    let mut inflight = 1;
    let mut collected_files = HashMap::new();
    let mut requested = HashSet::new();

    while inflight > 0 {
        let response = response_stream
            .message()
            .await
            .map_err(ReflectionError::StreamFailure)?
            .ok_or(ReflectionError::StreamClosed)?;

        inflight -= 1;

        match response.message_response {
            Some(MessageResponse::FileDescriptorResponse(res)) => {
                let sent_count = process_descriptor_batch(
                    res.file_descriptor_proto,
                    &mut collected_files,
                    &mut requested,
                    &request_channel,
                )
                .await?;

                inflight += sent_count;
            }
            Some(MessageResponse::ErrorResponse(e)) => {
                return Err(ReflectionError::ServerError {
                    message: e.error_message,
                    code: e.error_code,
                });
            }
            _ => {
                return Err(ReflectionError::StreamClosed);
            }
        }
    }

    Ok(collected_files)
}

async fn process_descriptor_batch(
    raw_protos: Vec<Vec<u8>>,
    collected_files: &mut HashMap<String, FileDescriptorProto>,
    requested: &mut HashSet<String>,
    tx: &mpsc::Sender<ServerReflectionRequest>,
) -> Result<usize, ReflectionError> {
    let mut sent_count = 0;

    for raw in raw_protos {
        let fd = FileDescriptorProto::decode(raw.as_ref())?;

        if let Some(name) = &fd.name
            && !collected_files.contains_key(name)
        {
            sent_count += queue_dependencies(&fd, collected_files, requested, tx).await?;
            collected_files.insert(name.clone(), fd);
        }
    }

    Ok(sent_count)
}

async fn queue_dependencies(
    fd: &FileDescriptorProto,
    collected_files: &HashMap<String, FileDescriptorProto>,
    requested: &mut HashSet<String>,
    tx: &mpsc::Sender<ServerReflectionRequest>,
) -> Result<usize, ReflectionError> {
    let mut count = 0;

    for dep in &fd.dependency {
        if !collected_files.contains_key(dep) && requested.insert(dep.clone()) {
            let req = ServerReflectionRequest {
                host: EMPTY_HOST.to_string(),
                message_request: Some(MessageRequest::FileByFilename(dep.clone())),
            };

            tx.send(req)
                .await
                .map_err(|_| ReflectionError::SendFailed)?;
            count += 1;
        }
    }

    Ok(count)
}
