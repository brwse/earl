#![cfg(feature = "http")]

use std::fmt::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[cfg(feature = "grpc")]
use earl::protocol::builder::PreparedGrpcData;
use earl::protocol::builder::{
    PreparedBody, PreparedHttpData, PreparedProtocolData, PreparedRequest,
};
use earl::protocol::executor::execute_prepared_request_with_host_validator;
use earl::protocol::transport::ResolvedTransport;
use earl::template::schema::{AllowRule, ResultDecode, ResultExtract, ResultTemplate};
use earl_core::Redactor;
use serde_json::{Map, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
#[cfg(feature = "grpc")]
use tokio_stream::wrappers::TcpListenerStream;
use url::Url;

fn loopback_resolver() -> Vec<IpAddr> {
    vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]
}

async fn spawn_test_server() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let retry_count = Arc::new(AtomicUsize::new(0));
    let retry_counter = retry_count.clone();

    tokio::spawn(async move {
        loop {
            let accepted = listener.accept().await;
            let Ok((mut socket, _)) = accepted else {
                break;
            };
            let retry_counter = retry_counter.clone();
            tokio::spawn(async move {
                let mut buf = [0_u8; 8192];
                let read = match socket.read(&mut buf).await {
                    Ok(n) => n,
                    Err(_) => return,
                };
                if read == 0 {
                    return;
                }

                let req = String::from_utf8_lossy(&buf[..read]);
                let first_line = req.lines().next().unwrap_or_default();
                let mut parts = first_line.split_whitespace();
                let method = parts.next().unwrap_or("GET");
                let path = parts.next().unwrap_or("/");

                let (status, headers, body) = match path {
                    "/redirect303" => (303, vec![("Location", "/final")], "".to_string()),
                    "/redirect302post" => {
                        (302, vec![("Location", "/final_method")], "".to_string())
                    }
                    "/final" => (
                        200,
                        vec![("Content-Type", "application/json")],
                        json!({"ok": true}).to_string(),
                    ),
                    "/final_method" => (
                        200,
                        vec![("Content-Type", "application/json")],
                        json!({"method": method}).to_string(),
                    ),
                    "/retry" => {
                        let attempt = retry_counter.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt == 1 {
                            (
                                503,
                                vec![("Content-Type", "text/plain")],
                                "retry".to_string(),
                            )
                        } else {
                            (
                                200,
                                vec![("Content-Type", "application/json")],
                                json!({"ok": true}).to_string(),
                            )
                        }
                    }
                    "/hop1" => (302, vec![("Location", "/hop2")], String::new()),
                    "/hop2" => (302, vec![("Location", "/hop3")], String::new()),
                    "/hop3" => (
                        200,
                        vec![("Content-Type", "application/json")],
                        json!({"ok": true}).to_string(),
                    ),
                    _ => (
                        404,
                        vec![("Content-Type", "text/plain")],
                        "not found".to_string(),
                    ),
                };

                let mut response = format!(
                    "HTTP/1.1 {} OK\r\ncontent-length: {}\r\n",
                    status,
                    body.len()
                );
                for (k, v) in headers {
                    let _ = write!(response, "{}: {}\r\n", k, v);
                }
                response.push_str("\r\n");
                response.push_str(&body);

                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    (format!("http://{}", addr), retry_count)
}

fn prepared_request(
    method: reqwest::Method,
    url: String,
    path_prefix: &str,
    result_template: ResultTemplate,
    transport: ResolvedTransport,
) -> PreparedRequest {
    let parsed = Url::parse(&url).unwrap();
    let host = parsed.host_str().unwrap().to_string();
    let port = parsed.port_or_known_default().unwrap();

    PreparedRequest {
        key: "test.command".to_string(),
        mode: earl::template::schema::CommandMode::Read,
        stream: false,
        allow_rules: vec![AllowRule {
            scheme: "http".to_string(),
            host,
            port,
            path_prefix: path_prefix.to_string(),
        }],
        allow_private_ips: false,
        transport,
        result_template,
        args: Map::new(),
        redactor: Redactor::default(),
        protocol_data: PreparedProtocolData::Http(PreparedHttpData {
            method,
            url: parsed,
            query: vec![],
            headers: vec![],
            cookies: vec![],
            body: PreparedBody::Empty,
        }),
    }
}

#[cfg(feature = "grpc")]
fn prepared_grpc_request(
    url: String,
    result_template: ResultTemplate,
    descriptor_set: Option<Vec<u8>>,
) -> PreparedRequest {
    let parsed = Url::parse(&url).unwrap();
    let host = parsed.host_str().unwrap().to_string();
    let port = parsed.port_or_known_default().unwrap();

    PreparedRequest {
        key: "test.grpc".to_string(),
        mode: earl::template::schema::CommandMode::Read,
        stream: false,
        allow_rules: vec![AllowRule {
            scheme: "http".to_string(),
            host,
            port,
            path_prefix: "/grpc.health.v1.Health/".to_string(),
        }],
        allow_private_ips: false,
        transport: ResolvedTransport {
            timeout: Duration::from_secs(5),
            follow_redirects: false,
            max_redirect_hops: 0,
            retry_max_attempts: 1,
            retry_backoff: Duration::from_millis(1),
            retry_on_status: vec![],
            compression: true,
            tls_min_version: None,
            proxy_url: None,
            max_response_bytes: 8 * 1024 * 1024,
        },
        result_template,
        args: Map::new(),
        redactor: Redactor::default(),
        protocol_data: PreparedProtocolData::Grpc(PreparedGrpcData {
            url: parsed,
            headers: vec![],
            body: PreparedBody::Json(json!({ "service": "" })),
            service: "grpc.health.v1.Health".to_string(),
            method: "Check".to_string(),
            descriptor_set,
        }),
    }
}

#[cfg(feature = "grpc")]
async fn spawn_grpc_health_server(with_reflection: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);
    let (_reporter, health_service) = tonic_health::server::health_reporter();

    tokio::spawn(async move {
        if with_reflection {
            let reflection_service = tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
                .build_v1()
                .unwrap();
            tonic::transport::Server::builder()
                .add_service(health_service)
                .add_service(reflection_service)
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        } else {
            tonic::transport::Server::builder()
                .add_service(health_service)
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        }
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn follows_redirect_for_302() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/method".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 3,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::POST,
        format!("{base_url}/redirect302post"),
        "/",
        result_template,
        transport,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(out.status, 200);
}

#[tokio::test]
async fn rewrites_post_to_get_for_302() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/method".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 3,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::POST,
        format!("{base_url}/redirect302post"),
        "/",
        result_template,
        transport,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(out.result, json!("GET"));
}

#[tokio::test]
async fn retries_on_configured_status_returns_200() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/ok".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 2,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![503],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/retry"),
        "/",
        result_template,
        transport,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(out.status, 200);
}

#[tokio::test]
async fn retries_on_configured_status_result_is_ok_true() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/ok".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 2,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![503],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/retry"),
        "/",
        result_template,
        transport,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(out.result, json!(true));
}

#[tokio::test]
async fn retries_on_configured_status_makes_two_attempts() {
    let (base_url, retry_counter) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 2,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![503],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/retry"),
        "/",
        result_template,
        transport,
    );

    execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(retry_counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn fails_when_redirect_hops_exceed_limit() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "ok".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 1,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/hop1"),
        "/",
        result_template,
        transport,
    );

    execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap_err();
}

#[tokio::test]
async fn blocks_request_when_allowlist_does_not_match() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "ok".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/final"),
        "/allowed-only",
        result_template,
        transport,
    );

    execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap_err();
}

#[tokio::test]
async fn empty_allowlist_returns_200() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/ok".to_string(),
        }),
        output: "ok".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let mut prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/final"),
        "/",
        result_template,
        transport,
    );
    prepared.allow_rules.clear();

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 200);
}

#[tokio::test]
async fn empty_allowlist_result_matches_response() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/ok".to_string(),
        }),
        output: "ok".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let mut prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/final"),
        "/",
        result_template,
        transport,
    );
    prepared.allow_rules.clear();

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.result, json!(true));
}

#[tokio::test]
async fn extracts_json_pointer_from_response_body() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/ok".to_string(),
        }),
        output: "ok".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/final"),
        "/",
        result_template,
        transport,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(out.result, json!(true));
}

#[tokio::test]
async fn decoded_field_contains_full_json_response() {
    let (base_url, _) = spawn_test_server().await;
    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/ok".to_string(),
        }),
        output: "ok".to_string(),
        result_alias: None,
    };

    let transport = ResolvedTransport {
        timeout: Duration::from_secs(5),
        follow_redirects: true,
        max_redirect_hops: 2,
        retry_max_attempts: 1,
        retry_backoff: Duration::from_millis(1),
        retry_on_status: vec![],
        compression: true,
        tls_min_version: None,
        proxy_url: None,
        max_response_bytes: 8 * 1024 * 1024,
    };

    let prepared = prepared_request(
        reqwest::Method::GET,
        format!("{base_url}/final"),
        "/",
        result_template,
        transport,
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();
    assert_eq!(out.decoded["ok"], json!(true));
}

#[tokio::test]
#[cfg(feature = "grpc")]
async fn grpc_check_with_reflection_returns_zero_status() {
    let base_url = spawn_grpc_health_server(true).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };
    let prepared = prepared_grpc_request(base_url, result_template, None);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
}

#[tokio::test]
#[cfg(feature = "grpc")]
async fn grpc_check_with_reflection_routes_to_health_check_endpoint() {
    let base_url = spawn_grpc_health_server(true).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };
    let prepared = prepared_grpc_request(base_url, result_template, None);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert!(out.url.contains("/grpc.health.v1.Health/Check"));
}

#[tokio::test]
#[cfg(feature = "grpc")]
async fn grpc_check_with_reflection_response_includes_status_field() {
    let base_url = spawn_grpc_health_server(true).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };
    let prepared = prepared_grpc_request(base_url, result_template, None);

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert!(out.decoded.get("status").is_some());
}

#[tokio::test]
#[cfg(feature = "grpc")]
async fn grpc_check_with_descriptor_set_returns_zero_status() {
    let base_url = spawn_grpc_health_server(false).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/status".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };
    let prepared = prepared_grpc_request(
        base_url,
        result_template,
        Some(tonic_health::pb::FILE_DESCRIPTOR_SET.to_vec()),
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert_eq!(out.status, 0);
}

#[tokio::test]
#[cfg(feature = "grpc")]
async fn grpc_check_with_descriptor_set_result_is_string() {
    let base_url = spawn_grpc_health_server(false).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: Some(ResultExtract::JsonPointer {
            json_pointer: "/status".to_string(),
        }),
        output: "{{ result }}".to_string(),
        result_alias: None,
    };
    let prepared = prepared_grpc_request(
        base_url,
        result_template,
        Some(tonic_health::pb::FILE_DESCRIPTOR_SET.to_vec()),
    );

    let out = execute_prepared_request_with_host_validator(&prepared, |_url| async {
        Ok(loopback_resolver())
    })
    .await
    .unwrap();

    assert!(out.result.is_string());
}
