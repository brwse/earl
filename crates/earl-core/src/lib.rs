pub mod allowlist;
pub mod decode;
pub mod redact;
pub mod render;
pub mod schema;
pub mod transport;

use std::future::Future;

use serde_json::{Map, Value};

pub use allowlist::ensure_url_allowed;
pub use decode::{DecodedBody, decode_response};
pub use redact::Redactor;
pub use render::TemplateRenderer;
pub use schema::{AllowRule, CommandMode, ResultDecode, ResultExtract, ResultTemplate};
pub use transport::ResolvedTransport;

/// Body data prepared for HTTP-like protocol execution.
#[derive(Debug, Clone)]
pub enum PreparedBody {
    Empty,
    Json(Value),
    Form(Vec<(String, String)>),
    Multipart(Vec<PreparedMultipartPart>),
    RawBytes {
        bytes: Vec<u8>,
        content_type: Option<String>,
    },
}

/// A single part in a multipart body.
#[derive(Debug, Clone)]
pub struct PreparedMultipartPart {
    pub name: String,
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
}

/// Unified result type returned by all protocol executors.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: u16,
    pub url: String,
    pub result: Value,
    pub decoded: Value,
}

/// Raw protocol output before decode/extract post-processing.
#[derive(Debug, Clone)]
pub struct RawExecutionResult {
    pub status: u16,
    pub url: String,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
}

/// A single chunk from a streaming response.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub data: Vec<u8>,
    pub content_type: Option<String>,
}

/// Metadata returned when a streaming execution completes.
#[derive(Debug, Clone)]
pub struct StreamMeta {
    pub status: u16,
    pub url: String,
}

/// Shared execution context passed to all protocol executors alongside
/// their protocol-specific prepared data.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub key: String,
    pub mode: CommandMode,
    pub allow_rules: Vec<AllowRule>,
    pub transport: ResolvedTransport,
    pub result_template: ResultTemplate,
    pub args: Map<String, Value>,
    pub redactor: Redactor,
}

/// Contract implemented by all protocol executors.
///
/// Each protocol crate provides an executor struct that implements this trait.
/// The associated `PreparedData` type links the executor to its matching
/// prepared data produced by the builder.
pub trait ProtocolExecutor {
    /// Protocol-specific prepared data (e.g. `PreparedHttpData`, `PreparedBashScript`).
    type PreparedData: Clone + std::fmt::Debug + Send + Sync;

    /// Execute a single protocol request and return the raw result.
    fn execute(
        &mut self,
        data: &Self::PreparedData,
        context: &ExecutionContext,
    ) -> impl Future<Output = anyhow::Result<RawExecutionResult>> + Send;
}

/// Contract for protocol executors that support streaming output.
///
/// Instead of buffering the full response, the executor sends individual
/// chunks through the provided `mpsc::Sender` as they arrive.
pub trait StreamingProtocolExecutor {
    /// Protocol-specific prepared data.
    type PreparedData: Clone + std::fmt::Debug + Send + Sync;

    /// Execute a streaming request, sending chunks through `sender`.
    /// Returns metadata about the completed stream.
    fn execute_stream(
        &mut self,
        data: &Self::PreparedData,
        context: &ExecutionContext,
        sender: tokio::sync::mpsc::Sender<StreamChunk>,
    ) -> impl Future<Output = anyhow::Result<StreamMeta>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_chunk_can_be_created() {
        let chunk = StreamChunk {
            data: b"hello".to_vec(),
            content_type: Some("application/json".to_string()),
        };
        assert_eq!(chunk.data, b"hello");
        assert_eq!(chunk.content_type.as_deref(), Some("application/json"));
    }

    #[test]
    fn stream_meta_can_be_created() {
        let meta = StreamMeta {
            status: 200,
            url: "https://example.com".to_string(),
        };
        assert_eq!(meta.status, 200);
    }
}

#[cfg(test)]
mod streaming_tests {
    use super::*;
    use serde_json::Map;
    use std::time::Duration;
    use tokio::sync::mpsc;

    struct MockStreamExecutor;

    impl StreamingProtocolExecutor for MockStreamExecutor {
        type PreparedData = String;

        async fn execute_stream(
            &mut self,
            _data: &String,
            _context: &ExecutionContext,
            sender: mpsc::Sender<StreamChunk>,
        ) -> anyhow::Result<StreamMeta> {
            sender
                .send(StreamChunk {
                    data: b"chunk1".to_vec(),
                    content_type: None,
                })
                .await
                .unwrap();
            Ok(StreamMeta {
                status: 200,
                url: "https://example.com".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn mock_streaming_executor_sends_chunks() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut executor = MockStreamExecutor;
        let context = ExecutionContext {
            key: "test".to_string(),
            mode: CommandMode::Read,
            allow_rules: vec![],
            transport: ResolvedTransport {
                timeout: Duration::from_secs(30),
                follow_redirects: true,
                max_redirect_hops: 10,
                retry_max_attempts: 0,
                retry_backoff: Duration::from_millis(100),
                retry_on_status: vec![],
                compression: false,
                tls_min_version: None,
                proxy_url: None,
                max_response_bytes: 10_000_000,
            },
            result_template: ResultTemplate::default(),
            args: Map::new(),
            redactor: Redactor::new(vec![]),
        };

        let meta = executor
            .execute_stream(&"test".to_string(), &context, tx)
            .await
            .unwrap();

        assert_eq!(meta.status, 200);
        let chunk = rx.recv().await.unwrap();
        assert_eq!(chunk.data, b"chunk1");
    }
}
