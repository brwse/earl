use std::future::Future;
use std::net::{IpAddr, SocketAddr};

use anyhow::{Context, Result, bail};
use reqwest::header::{CONTENT_TYPE, COOKIE, HeaderMap, HeaderName, HeaderValue, LOCATION};
use url::Url;

use earl_core::allowlist::ensure_url_allowed;
use earl_core::{ExecutionContext, PreparedBody, PreparedMultipartPart, RawExecutionResult};

use crate::PreparedHttpData;

/// Execute a single HTTP request (with redirect following) and return the result.
pub async fn execute_http_once_with_host_validator<F, Fut>(
    http_data: &PreparedHttpData,
    ctx: &ExecutionContext,
    host_validator: &mut F,
) -> Result<RawExecutionResult>
where
    F: FnMut(Url) -> Fut,
    Fut: Future<Output = Result<Vec<IpAddr>>>,
{
    let mut method = http_data.method.clone();
    let mut body = http_data.body.clone();
    let mut url = http_data.url.clone();

    for hop in 0..=ctx.transport.max_redirect_hops {
        ensure_url_allowed(&url, &ctx.allow_rules)?;
        let resolved_ips = host_validator(url.clone()).await?;
        let client = build_http_client(ctx, &url, &resolved_ips)?;

        let request = build_request(
            &client,
            &method,
            &url,
            &http_data.headers,
            &http_data.cookies,
            &http_data.query,
            &body,
        )?;
        let response = request
            .send()
            .await
            .with_context(|| format!("request execution failed for `{}`", url.as_str()))?;

        if response.status().is_redirection() && ctx.transport.follow_redirects {
            if hop >= ctx.transport.max_redirect_hops {
                bail!(
                    "maximum redirect hops reached ({})",
                    ctx.transport.max_redirect_hops
                );
            }

            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| anyhow::anyhow!("redirect response missing Location header"))?
                .to_str()
                .context("redirect Location header is not valid UTF-8")?
                .to_string();

            let new_url = url
                .join(&location)
                .with_context(|| format!("invalid redirect Location `{location}`"))?;

            let status = response.status().as_u16();
            if status == 303
                || ((status == 301 || status == 302) && method == reqwest::Method::POST)
            {
                method = reqwest::Method::GET;
                body = PreparedBody::Empty;
            }
            url = new_url;
            continue;
        }

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string());

        let body_bytes =
            read_response_body_limited(response, ctx.transport.max_response_bytes).await?;

        return Ok(RawExecutionResult {
            status,
            url: url.to_string(),
            body: body_bytes,
            content_type,
        });
    }

    bail!("redirect handling failed unexpectedly")
}

fn build_request(
    client: &reqwest::Client,
    method: &reqwest::Method,
    url: &Url,
    headers: &[(String, String)],
    cookies: &[(String, String)],
    query: &[(String, String)],
    body: &PreparedBody,
) -> Result<reqwest::RequestBuilder> {
    let mut builder = client.request(method.clone(), url.clone());

    if !query.is_empty() {
        builder = builder.query(query);
    }

    let mut header_map = HeaderMap::new();
    for (name, value) in headers {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid header name `{name}`"))?;
        let header_value = HeaderValue::from_str(value)
            .with_context(|| format!("invalid header value for `{name}`"))?;
        header_map.append(header_name, header_value);
    }

    if !cookies.is_empty() {
        let cookie_value = cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        header_map.insert(
            COOKIE,
            HeaderValue::from_str(&cookie_value).context("invalid cookie header value")?,
        );
    }

    builder = builder.headers(header_map);

    match body {
        PreparedBody::Empty => {}
        PreparedBody::Json(value) => {
            builder = builder.json(value);
        }
        PreparedBody::Form(fields) => {
            builder = builder.form(fields);
        }
        PreparedBody::Multipart(parts) => {
            builder = builder.multipart(build_multipart(parts)?);
        }
        PreparedBody::RawBytes {
            bytes,
            content_type,
        } => {
            if let Some(content_type) = content_type {
                builder = builder.header(CONTENT_TYPE, content_type);
            }
            builder = builder.body(bytes.clone());
        }
    }

    Ok(builder)
}

fn build_http_client(
    ctx: &ExecutionContext,
    url: &Url,
    resolved_ips: &[IpAddr],
) -> Result<reqwest::Client> {
    if resolved_ips.is_empty() {
        bail!("host validation returned no resolved IP addresses");
    }

    let mut builder = reqwest::Client::builder()
        .timeout(ctx.transport.timeout)
        .redirect(reqwest::redirect::Policy::none())
        .gzip(ctx.transport.compression)
        .brotli(ctx.transport.compression)
        .zstd(ctx.transport.compression)
        .deflate(ctx.transport.compression);

    if let Some(version) = ctx.transport.tls_min_version {
        builder = builder.min_tls_version(version);
    }

    if let Some(proxy_url) = &ctx.transport.proxy_url {
        let proxy = reqwest::Proxy::all(proxy_url)
            .with_context(|| format!("invalid proxy URL `{proxy_url}`"))?;
        builder = builder.proxy(proxy);
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("request URL missing host"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("request URL missing port"))?;

    if !resolved_ips.is_empty() {
        let addrs: Vec<SocketAddr> = resolved_ips
            .iter()
            .map(|ip| SocketAddr::new(*ip, port))
            .collect();
        builder = builder.resolve_to_addrs(host, &addrs);
    }

    builder
        .build()
        .context("failed constructing reqwest client")
}

async fn read_response_body_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if out.len().saturating_add(chunk.len()) > limit {
            bail!("response body exceeded configured max_response_bytes ({limit} bytes)");
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

use earl_core::{ProtocolExecutor, StreamChunk, StreamMeta, StreamingProtocolExecutor};
use tokio::sync::mpsc;

/// HTTP/GraphQL protocol executor.
///
/// Holds a host validator closure used for DNS resolution and SSRF protection.
pub struct HttpExecutor<F> {
    pub host_validator: F,
}

impl<F, Fut> ProtocolExecutor for HttpExecutor<F>
where
    F: FnMut(Url) -> Fut + Send,
    Fut: Future<Output = Result<Vec<IpAddr>>> + Send,
{
    type PreparedData = PreparedHttpData;

    async fn execute(
        &mut self,
        data: &PreparedHttpData,
        ctx: &ExecutionContext,
    ) -> Result<RawExecutionResult> {
        execute_http_once_with_host_validator(data, ctx, &mut self.host_validator).await
    }
}

/// Streaming HTTP executor — sends response chunks as they arrive.
///
/// Reuses the same connection setup (redirect following, SSRF validation,
/// client building) as [`HttpExecutor`] but streams chunks through an
/// `mpsc::Sender` instead of buffering the entire response body.
pub struct HttpStreamExecutor<F> {
    pub host_validator: F,
}

impl<F, Fut> StreamingProtocolExecutor for HttpStreamExecutor<F>
where
    F: FnMut(Url) -> Fut + Send,
    Fut: Future<Output = Result<Vec<IpAddr>>> + Send,
{
    type PreparedData = PreparedHttpData;

    async fn execute_stream(
        &mut self,
        data: &PreparedHttpData,
        ctx: &ExecutionContext,
        sender: mpsc::Sender<StreamChunk>,
    ) -> anyhow::Result<StreamMeta> {
        let mut method = data.method.clone();
        let mut body = data.body.clone();
        let mut url = data.url.clone();

        for hop in 0..=ctx.transport.max_redirect_hops {
            ensure_url_allowed(&url, &ctx.allow_rules)?;
            let resolved_ips = (self.host_validator)(url.clone()).await?;
            let client = build_http_client(ctx, &url, &resolved_ips)?;

            let request = build_request(
                &client,
                &method,
                &url,
                &data.headers,
                &data.cookies,
                &data.query,
                &body,
            )?;
            let response = request
                .send()
                .await
                .with_context(|| format!("request execution failed for `{}`", url.as_str()))?;

            if response.status().is_redirection() && ctx.transport.follow_redirects {
                if hop >= ctx.transport.max_redirect_hops {
                    bail!(
                        "maximum redirect hops reached ({})",
                        ctx.transport.max_redirect_hops
                    );
                }

                let location = response
                    .headers()
                    .get(LOCATION)
                    .ok_or_else(|| anyhow::anyhow!("redirect response missing Location header"))?
                    .to_str()
                    .context("redirect Location header is not valid UTF-8")?
                    .to_string();

                let new_url = url
                    .join(&location)
                    .with_context(|| format!("invalid redirect Location `{location}`"))?;

                let status = response.status().as_u16();
                if status == 303
                    || ((status == 301 || status == 302) && method == reqwest::Method::POST)
                {
                    method = reqwest::Method::GET;
                    body = PreparedBody::Empty;
                }
                url = new_url;
                continue;
            }

            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_string());

            // Stream chunks instead of buffering the entire response body.
            let mut response = response;
            let mut total_bytes = 0usize;
            while let Some(chunk) = response.chunk().await? {
                total_bytes = total_bytes.saturating_add(chunk.len());
                if total_bytes > ctx.transport.max_response_bytes {
                    bail!(
                        "streaming response exceeded configured max_response_bytes ({} bytes)",
                        ctx.transport.max_response_bytes
                    );
                }
                let stream_chunk = StreamChunk {
                    data: chunk.to_vec(),
                    content_type: content_type.clone(),
                };
                if sender.send(stream_chunk).await.is_err() {
                    // Receiver dropped — stop streaming gracefully.
                    break;
                }
            }

            return Ok(StreamMeta {
                status,
                url: url.to_string(),
            });
        }

        bail!("redirect handling failed unexpectedly")
    }
}

fn build_multipart(parts: &[PreparedMultipartPart]) -> Result<reqwest::multipart::Form> {
    let mut form = reqwest::multipart::Form::new();
    for part in parts {
        let mut req_part = reqwest::multipart::Part::bytes(part.bytes.clone());
        if let Some(content_type) = &part.content_type {
            req_part = req_part.mime_str(content_type)?;
        }
        if let Some(filename) = &part.filename {
            req_part = req_part.file_name(filename.clone());
        }
        form = form.part(part.name.clone(), req_part);
    }
    Ok(form)
}
