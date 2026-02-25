// tests/common/server.rs

#![allow(dead_code)]

use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// A single route's response.
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
    /// Extra headers like "Set-Cookie" or "Location"
    pub extra_headers: Vec<(&'static str, String)>,
}

impl Response {
    pub fn ok(content_type: &'static str, body: impl Into<String>) -> Self {
        Self { status: 200, content_type, body: body.into(), extra_headers: vec![] }
    }

    pub fn html(body: impl Into<String>) -> Self {
        Self::ok("text/html", body)
    }

    pub fn redirect(location: impl Into<String>) -> Self {
        Self {
            status: 302,
            content_type: "text/plain",
            body: String::new(),
            extra_headers: vec![("Location", location.into())],
        }
    }

    pub fn with_cookie(mut self, cookie: impl Into<String>) -> Self {
        self.extra_headers.push(("Set-Cookie", cookie.into()));
        self
    }
}

pub struct TestServer {
    pub port: u16,
    abort: tokio::task::AbortHandle,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

impl TestServer {
    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }
}

/// Spawn a local HTTP/1.1 server.
///
/// `routes` maps `"METHOD /path"` (e.g. `"GET /login"`, `"POST /submit"`) to a
/// `Response`.  Unknown routes return 404.  The server is shut down when the
/// returned `TestServer` is dropped.
pub async fn spawn(routes: HashMap<String, Response>) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    use std::sync::Arc;
    let routes = Arc::new(routes);

    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { break };
            let routes = Arc::clone(&routes);
            tokio::spawn(async move {
                handle_connection(stream, routes).await;
            });
        }
    });

    TestServer { port, abort: task.abort_handle() }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    routes: std::sync::Arc<HashMap<String, Response>>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read request line.
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await.is_err() { return; }
    let parts: Vec<&str> = request_line.trim().splitn(3, ' ').collect();
    if parts.len() < 2 { return; }
    let method = parts[0].to_uppercase();
    let path = parts[1].to_string();

    // Drain headers.
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.is_err() { return; }
        let line = line.trim();
        if line.is_empty() { break; }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse().unwrap_or(0);
        }
    }

    // Read body for POST.
    let _body = if content_length > 0 {
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; content_length];
        let _ = reader.read_exact(&mut buf).await;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    let key = format!("{} {}", method, path);
    let response = routes.get(&key).or_else(|| routes.get(&format!("ANY {}", path)));

    let (status, status_text, content_type, body, extra_headers) = match response {
        Some(r) => (r.status, status_text(r.status), r.content_type, r.body.clone(), &r.extra_headers[..]),
        None => (404, "Not Found", "text/plain", "Not Found".to_string(), [].as_slice()),
    };

    let mut resp = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in extra_headers {
        resp.push_str(&format!("{k}: {v}\r\n"));
    }
    resp.push_str("\r\n");
    resp.push_str(&body);

    let _ = writer.write_all(resp.as_bytes()).await;
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        404 => "Not Found",
        _ => "Unknown",
    }
}
