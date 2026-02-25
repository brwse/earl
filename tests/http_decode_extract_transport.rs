use std::collections::BTreeMap;

use earl::protocol::extract::extract_result;
use earl::protocol::transport::{ResolvedTransport, resolve_transport};
use earl::template::schema::{
    RedirectTemplate, ResultDecode, ResultExtract, RetryTemplate, TlsTemplate, TransportTemplate,
};
use earl_core::decode::{DecodedBody, decode_response};
use serde_json::json;

#[test]
fn auto_decode_json_content_type_returns_json_body() {
    let json_bytes = br#"{"ok":true}"#;
    let decoded =
        decode_response(ResultDecode::Auto, Some("application/json"), json_bytes).unwrap();
    let DecodedBody::Json(value) = decoded else { panic!("expected JSON") };
    assert_eq!(value["ok"], json!(true));
}

#[test]
fn auto_decode_text_content_type_returns_text_body() {
    let text_bytes = b"hello";
    let decoded = decode_response(ResultDecode::Auto, Some("text/plain"), text_bytes).unwrap();
    let DecodedBody::Text(value) = decoded else { panic!("expected text") };
    assert_eq!(value, "hello");
}

#[test]
fn binary_body_base64_encodes_when_converted_to_json() {
    let bytes = vec![1_u8, 2, 3, 4];
    let decoded = decode_response(
        ResultDecode::Binary,
        Some("application/octet-stream"),
        &bytes,
    )
    .unwrap();
    let as_json = decoded.to_json_value();
    assert_eq!(as_json, json!("AQIDBA=="));
}

#[test]
fn json_pointer_extracts_nested_value() {
    let decoded_json = DecodedBody::Json(json!({"data": {"id": 42}}));
    let out = extract_result(
        Some(&ResultExtract::JsonPointer {
            json_pointer: "/data/id".to_string(),
        }),
        &decoded_json,
    )
    .unwrap();
    assert_eq!(out, json!(42));
}

#[test]
fn regex_extracts_capture_group_from_text() {
    let decoded_text = DecodedBody::Text("id=abc-123".to_string());
    let out = extract_result(
        Some(&ResultExtract::Regex {
            regex: "id=([a-z0-9-]+)".to_string(),
        }),
        &decoded_text,
    )
    .unwrap();
    assert_eq!(out, json!("abc-123"));
}

#[test]
fn css_selector_extracts_all_matching_elements() {
    let decoded_html =
        DecodedBody::Html("<html><body><h1>Hello</h1><h1>World</h1></body></html>".to_string());
    let out = extract_result(
        Some(&ResultExtract::CssSelector {
            css_selector: "h1".to_string(),
        }),
        &decoded_html,
    )
    .unwrap();
    assert_eq!(out, json!(["Hello", "World"]));
}

#[test]
fn xpath_extracts_text_nodes() {
    let decoded_xml = DecodedBody::Xml("<root><item>A</item><item>B</item></root>".to_string());
    let out = extract_result(
        Some(&ResultExtract::XPath {
            xpath: "//item/text()".to_string(),
        }),
        &decoded_xml,
    )
    .unwrap();
    assert_eq!(out, json!(["A", "B"]));
}

#[test]
fn json_pointer_on_missing_key_returns_error() {
    let decoded_json = DecodedBody::Json(json!({"a": 1}));
    extract_result(
        Some(&ResultExtract::JsonPointer {
            json_pointer: "/missing".to_string(),
        }),
        &decoded_json,
    )
    .unwrap_err();
}

#[test]
fn default_transport_retry_max_attempts_is_one() {
    let defaults = resolve_transport(None, &BTreeMap::new()).unwrap();
    assert_eq!(defaults.retry_max_attempts, 1);
}

#[test]
fn default_transport_max_redirect_hops_is_five() {
    let defaults = resolve_transport(None, &BTreeMap::new()).unwrap();
    assert_eq!(defaults.max_redirect_hops, 5);
}

#[test]
fn default_transport_compression_is_enabled() {
    let defaults = resolve_transport(None, &BTreeMap::new()).unwrap();
    assert!(defaults.compression);
}

#[test]
fn default_transport_max_response_bytes_is_eight_mib() {
    let defaults = resolve_transport(None, &BTreeMap::new()).unwrap();
    assert_eq!(defaults.max_response_bytes, 8 * 1024 * 1024);
}

fn resolved_override_transport() -> ResolvedTransport {
    let override_input = TransportTemplate {
        timeout_ms: Some(2_000),
        max_response_bytes: Some(16 * 1024),
        redirects: Some(RedirectTemplate {
            follow: false,
            max_hops: 2,
        }),
        retry: Some(RetryTemplate {
            max_attempts: 0,
            backoff_ms: 0,
            retry_on_status: vec![429, 500],
        }),
        compression: Some(true),
        tls: Some(TlsTemplate {
            min_version: Some("1.2".to_string()),
        }),
        proxy_profile: Some("corp".to_string()),
    };

    let proxy_profiles = BTreeMap::from([(
        "corp".to_string(),
        earl::config::ProxyProfile {
            url: "http://127.0.0.1:8888".to_string(),
        },
    )]);

    resolve_transport(Some(&override_input), &proxy_profiles).unwrap()
}

#[test]
fn transport_timeout_resolved_from_template() {
    assert_eq!(resolved_override_transport().timeout.as_millis(), 2_000);
}

#[test]
fn transport_follow_redirects_disabled_when_template_sets_follow_false() {
    assert!(!resolved_override_transport().follow_redirects);
}

#[test]
fn transport_max_redirect_hops_resolved_from_template() {
    assert_eq!(resolved_override_transport().max_redirect_hops, 2);
}

#[test]
fn transport_retry_max_attempts_clamped_to_minimum() {
    assert_eq!(resolved_override_transport().retry_max_attempts, 1);
}

#[test]
fn transport_retry_on_status_resolved_from_template() {
    assert_eq!(resolved_override_transport().retry_on_status, vec![429, 500]);
}

#[test]
fn transport_compression_resolved_from_template() {
    assert!(resolved_override_transport().compression);
}

#[test]
fn transport_max_response_bytes_resolved_from_template() {
    assert_eq!(resolved_override_transport().max_response_bytes, 16 * 1024);
}

#[test]
fn transport_proxy_url_resolved_from_profile() {
    assert_eq!(
        resolved_override_transport().proxy_url.as_deref(),
        Some("http://127.0.0.1:8888")
    );
}

#[test]
fn transport_tls_min_version_resolved_from_template() {
    assert_eq!(
        resolved_override_transport().tls_min_version,
        Some(reqwest::tls::Version::TLS_1_2)
    );
}
