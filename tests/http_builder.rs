#![cfg(feature = "http")]

mod common;

use std::collections::BTreeMap;

use earl::config::SandboxConfig;
use earl::protocol::builder::{
    PreparedBody, PreparedProtocolData, build_prepared_request_with_token_provider,
};
use earl::template::catalog::{TemplateCatalogEntry, TemplateScope, TemplateSource};
use earl::template::schema::{
    AllowRule, Annotations, ApiKeyLocation, AuthTemplate, BodyTemplate, CommandMode,
    CommandTemplate, HttpOperationTemplate, MultipartPartTemplate, OperationTemplate, ResultDecode,
    ResultTemplate,
};
#[cfg(feature = "graphql")]
use earl::template::schema::{GraphqlOperationTemplate, GraphqlTemplate};
#[cfg(feature = "grpc")]
use earl::template::schema::{GrpcOperationTemplate, GrpcTemplate};
use secrecy::SecretString;
use serde_json::{Map, json};

fn base_entry(
    auth: Option<AuthTemplate>,
    body: Option<BodyTemplate>,
    secrets: Vec<&str>,
) -> TemplateCatalogEntry {
    TemplateCatalogEntry {
        key: "provider.command".to_string(),
        provider: "provider".to_string(),
        command: "command".to_string(),
        title: "Test Command".to_string(),
        summary: "Run test command".to_string(),
        description: "test".to_string(),
        categories: vec!["test".to_string()],
        mode: CommandMode::Read,
        source: TemplateSource {
            path: std::path::PathBuf::from("test.hcl"),
            scope: TemplateScope::Local,
        },
        template: CommandTemplate {
            title: "Test Command".to_string(),
            summary: "Run test command".to_string(),
            description: "test".to_string(),
            categories: vec![],
            annotations: Annotations {
                mode: CommandMode::Read,
                secrets: secrets.into_iter().map(ToString::to_string).collect(),
                allow_environment_protocol_switching: false,
            },
            params: vec![],
            operation: OperationTemplate::Http(HttpOperationTemplate {
                method: "POST".to_string(),
                url: "https://api.example.com/resource".to_string(),
                path: None,
                query: Some(BTreeMap::new()),
                headers: Some(BTreeMap::new()),
                cookies: Some(BTreeMap::new()),
                auth,
                body,
                stream: false,
                transport: None,
            }),
            result: ResultTemplate {
                decode: ResultDecode::Auto,
                extract: None,
                output: "ok".to_string(),
                result_alias: None,
            },
            environment_overrides: BTreeMap::new(),
        },
    }
}

fn default_allow_rules() -> Vec<AllowRule> {
    vec![AllowRule {
        scheme: "https".to_string(),
        host: "api.example.com".to_string(),
        port: 443,
        path_prefix: "/".to_string(),
    }]
}

fn default_proxy_profiles() -> BTreeMap<String, earl::config::ProxyProfile> {
    BTreeMap::new()
}

fn empty_args() -> Map<String, serde_json::Value> {
    Map::new()
}

#[tokio::test]
async fn builds_api_key_in_all_locations() {
    let ws = common::temp_workspace();
    let manager =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    manager
        .set("api.key", SecretString::new("secret123".to_string().into()))
        .unwrap();

    for location in [
        ApiKeyLocation::Header,
        ApiKeyLocation::Query,
        ApiKeyLocation::Cookie,
    ] {
        let entry = base_entry(
            Some(AuthTemplate::ApiKey {
                location: location.clone(),
                name: "X-Token".to_string(),
                secret: "api.key".to_string(),
            }),
            None,
            vec!["api.key"],
        );

        let prepared = build_prepared_request_with_token_provider(
            &entry,
            empty_args(),
            &manager,
            |_profile| async { Ok("unused".to_string()) },
            &default_allow_rules(),
            &default_proxy_profiles(),
            &SandboxConfig::default(),
        )
        .await
        .unwrap();

        let redacted = prepared.redactor.redact("secret123");
        assert_eq!(redacted, "[REDACTED]");

        let http_data = match &prepared.protocol_data {
            PreparedProtocolData::Http(data) => data,
            _ => panic!("expected Http protocol data"),
        };

        match location {
            ApiKeyLocation::Header => assert!(
                http_data
                    .headers
                    .iter()
                    .any(|(k, v)| k == "X-Token" && v == "secret123")
            ),
            ApiKeyLocation::Query => assert!(
                http_data
                    .query
                    .iter()
                    .any(|(k, v)| k == "X-Token" && v == "secret123")
            ),
            ApiKeyLocation::Cookie => assert!(
                http_data
                    .cookies
                    .iter()
                    .any(|(k, v)| k == "X-Token" && v == "secret123")
            ),
        }
    }
}

#[tokio::test]
async fn builds_bearer_basic_and_oauth_profile_auth() {
    let ws = common::temp_workspace();
    let manager =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    manager
        .set(
            "bearer.token",
            SecretString::new("bearer-123".to_string().into()),
        )
        .unwrap();
    manager
        .set(
            "basic.password",
            SecretString::new("pw-123".to_string().into()),
        )
        .unwrap();

    let bearer_entry = base_entry(
        Some(AuthTemplate::Bearer {
            secret: "bearer.token".to_string(),
        }),
        None,
        vec!["bearer.token"],
    );
    let bearer = build_prepared_request_with_token_provider(
        &bearer_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("oauth-token".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    let bearer_http = match &bearer.protocol_data {
        PreparedProtocolData::Http(data) => data,
        _ => panic!("expected Http protocol data"),
    };
    assert!(
        bearer_http
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer bearer-123")
    );

    let basic_entry = base_entry(
        Some(AuthTemplate::Basic {
            username: "alice".to_string(),
            password_secret: "basic.password".to_string(),
        }),
        None,
        vec!["basic.password"],
    );
    let basic = build_prepared_request_with_token_provider(
        &basic_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("oauth-token".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    let basic_http = match &basic.protocol_data {
        PreparedProtocolData::Http(data) => data,
        _ => panic!("expected Http protocol data"),
    };
    assert!(
        basic_http
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v.starts_with("Basic "))
    );

    let oauth_entry = base_entry(
        Some(AuthTemplate::OAuth2Profile {
            profile: "github".to_string(),
        }),
        None,
        vec![],
    );
    let oauth = build_prepared_request_with_token_provider(
        &oauth_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("oauth-token".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    let oauth_http = match &oauth.protocol_data {
        PreparedProtocolData::Http(data) => data,
        _ => panic!("expected Http protocol data"),
    };
    assert!(
        oauth_http
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer oauth-token")
    );
}

#[tokio::test]
async fn builds_json_form_and_raw_body_modes() {
    let ws = common::temp_workspace();
    let manager =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    let json_entry = base_entry(
        None,
        Some(BodyTemplate::Json {
            value: json!({"a": "{{ args.a }}"}),
        }),
        vec![],
    );
    let mut args = Map::new();
    args.insert("a".to_string(), json!("x"));
    let prepared = build_prepared_request_with_token_provider(
        &json_entry,
        args,
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    match &prepared.protocol_data {
        PreparedProtocolData::Http(data) => match &data.body {
            PreparedBody::Json(value) => assert_eq!(*value, json!({"a": "x"})),
            _ => panic!("expected json body"),
        },
        _ => panic!("expected Http protocol data"),
    }

    let form_entry = base_entry(
        None,
        Some(BodyTemplate::FormUrlencoded {
            fields: BTreeMap::from([
                ("q".to_string(), json!("test")),
                ("tags".to_string(), json!(["a", "b"])),
            ]),
        }),
        vec![],
    );
    let prepared = build_prepared_request_with_token_provider(
        &form_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    match &prepared.protocol_data {
        PreparedProtocolData::Http(data) => match &data.body {
            PreparedBody::Form(values) => {
                assert!(values.iter().any(|(k, v)| k == "q" && v == "test"));
                assert_eq!(values.iter().filter(|(k, _)| k == "tags").count(), 2);
            }
            _ => panic!("expected form body"),
        },
        _ => panic!("expected Http protocol data"),
    }

    let raw_entry = base_entry(
        None,
        Some(BodyTemplate::RawText {
            value: "hello".to_string(),
            content_type: None,
        }),
        vec![],
    );
    let prepared = build_prepared_request_with_token_provider(
        &raw_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    match &prepared.protocol_data {
        PreparedProtocolData::Http(data) => match &data.body {
            PreparedBody::RawBytes {
                bytes,
                content_type,
            } => {
                assert_eq!(bytes, b"hello");
                assert_eq!(content_type.as_deref(), Some("text/plain"));
            }
            _ => panic!("expected raw body"),
        },
        _ => panic!("expected Http protocol data"),
    }
}

#[tokio::test]
async fn builds_multipart_raw_bytes_and_file_stream_bodies() {
    let ws = common::temp_workspace();
    let manager =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    let file_path = ws.root.path().join("payload.txt");
    std::fs::write(&file_path, b"file-data").unwrap();

    let multipart_entry = base_entry(
        None,
        Some(BodyTemplate::Multipart {
            parts: vec![
                MultipartPartTemplate {
                    name: "inline".to_string(),
                    value: Some("hello".to_string()),
                    bytes_base64: None,
                    file_path: None,
                    content_type: Some("text/plain".to_string()),
                    filename: Some("inline.txt".to_string()),
                },
                MultipartPartTemplate {
                    name: "from_file".to_string(),
                    value: None,
                    bytes_base64: None,
                    file_path: Some(file_path.to_string_lossy().to_string()),
                    content_type: None,
                    filename: None,
                },
            ],
        }),
        vec![],
    );
    let prepared = build_prepared_request_with_token_provider(
        &multipart_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    match &prepared.protocol_data {
        PreparedProtocolData::Http(data) => match &data.body {
            PreparedBody::Multipart(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].bytes, b"hello");
                assert_eq!(parts[1].bytes, b"file-data");
            }
            _ => panic!("expected multipart body"),
        },
        _ => panic!("expected Http protocol data"),
    }

    let raw_bytes_entry = base_entry(
        None,
        Some(BodyTemplate::RawBytesBase64 {
            value: "aGVsbG8=".to_string(),
            content_type: Some("application/octet-stream".to_string()),
        }),
        vec![],
    );
    let prepared = build_prepared_request_with_token_provider(
        &raw_bytes_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    match &prepared.protocol_data {
        PreparedProtocolData::Http(data) => match &data.body {
            PreparedBody::RawBytes { bytes, .. } => assert_eq!(bytes, b"hello"),
            _ => panic!("expected raw bytes body"),
        },
        _ => panic!("expected Http protocol data"),
    }

    let file_stream_entry = base_entry(
        None,
        Some(BodyTemplate::FileStream {
            path: file_path.to_string_lossy().to_string(),
            content_type: Some("text/plain".to_string()),
        }),
        vec![],
    );
    let prepared = build_prepared_request_with_token_provider(
        &file_stream_entry,
        empty_args(),
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();
    match &prepared.protocol_data {
        PreparedProtocolData::Http(data) => match &data.body {
            PreparedBody::RawBytes { bytes, .. } => assert_eq!(bytes, b"file-data"),
            _ => panic!("expected file stream body"),
        },
        _ => panic!("expected Http protocol data"),
    }
}

#[tokio::test]
#[cfg(feature = "graphql")]
async fn builds_graphql_payload_and_headers() {
    let ws = common::temp_workspace();
    let manager =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));

    let mut entry = base_entry(None, None, vec![]);
    entry.template.operation = OperationTemplate::Graphql(GraphqlOperationTemplate {
        method: String::new(),
        url: "https://api.example.com/resource".to_string(),
        path: None,
        query: Some(BTreeMap::new()),
        headers: Some(BTreeMap::new()),
        cookies: Some(BTreeMap::new()),
        auth: None,
        graphql: GraphqlTemplate {
            query: "query User($id: ID!) { user(id: $id) { login } }".to_string(),
            operation_name: Some("User".to_string()),
            variables: Some(json!({ "id": "{{ args.user_id }}" })),
        },
        stream: false,
        transport: None,
    });

    let mut args = Map::new();
    args.insert("user_id".to_string(), json!("42"));

    let prepared = build_prepared_request_with_token_provider(
        &entry,
        args,
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();

    let graphql_data = match &prepared.protocol_data {
        PreparedProtocolData::Graphql(data) => data,
        _ => panic!("expected Graphql protocol data"),
    };
    assert_eq!(graphql_data.method, reqwest::Method::POST);

    match &graphql_data.body {
        PreparedBody::Json(payload) => {
            assert_eq!(
                *payload,
                json!({
                    "query": "query User($id: ID!) { user(id: $id) { login } }",
                    "operationName": "User",
                    "variables": {"id": 42}
                })
            );
        }
        _ => panic!("expected graphql payload in JSON body"),
    }

    assert!(
        graphql_data
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Content-Type") && v == "application/json")
    );
    assert!(
        graphql_data
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Accept") && v == "application/json")
    );
}

#[tokio::test]
#[cfg(feature = "grpc")]
async fn builds_grpc_payload_and_headers() {
    let ws = common::temp_workspace();
    let manager =
        common::in_memory_secret_manager(&ws.root.path().join("state/secrets-index.json"));
    manager
        .set(
            "grpc.token",
            SecretString::new("grpc-token".to_string().into()),
        )
        .unwrap();

    let mut entry = base_entry(None, None, vec!["grpc.token"]);
    entry.template.operation = OperationTemplate::Grpc(GrpcOperationTemplate {
        url: "http://127.0.0.1:50051".to_string(),
        headers: Some(BTreeMap::from([(
            "x-trace-id".to_string(),
            json!("{{ args.trace }}"),
        )])),
        auth: Some(AuthTemplate::Bearer {
            secret: "grpc.token".to_string(),
        }),
        grpc: GrpcTemplate {
            service: "grpc.health.v1.Health".to_string(),
            method: "Check".to_string(),
            body: Some(json!({
                "service": "{{ args.service }}"
            })),
            descriptor_set_file: None,
        },
        stream: false,
        transport: None,
    });

    let mut args = Map::new();
    args.insert("service".to_string(), json!(""));
    args.insert("trace".to_string(), json!("trace-123"));

    let prepared = build_prepared_request_with_token_provider(
        &entry,
        args,
        &manager,
        |_profile| async { Ok("unused".to_string()) },
        &default_allow_rules(),
        &default_proxy_profiles(),
        &SandboxConfig::default(),
    )
    .await
    .unwrap();

    let grpc_data = match &prepared.protocol_data {
        PreparedProtocolData::Grpc(data) => data,
        _ => panic!("expected Grpc protocol data"),
    };
    assert!(
        grpc_data
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer grpc-token")
    );
    assert!(
        grpc_data
            .headers
            .iter()
            .any(|(k, v)| k == "x-trace-id" && v == "trace-123")
    );
    assert_eq!(prepared.redactor.redact("grpc-token"), "[REDACTED]");

    match &grpc_data.body {
        PreparedBody::Json(value) => {
            assert_eq!(*value, json!({"service": ""}));
        }
        _ => panic!("expected grpc payload in JSON body"),
    }

    assert_eq!(grpc_data.service, "grpc.health.v1.Health");
    assert_eq!(grpc_data.method, "Check");
    assert!(grpc_data.descriptor_set.is_none());
}
