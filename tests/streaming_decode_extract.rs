//! Integration tests for the streaming decode/extract pipeline.
//!
//! These tests verify that individual [`StreamChunk`] values can be decoded and
//! extracted independently — the core property that makes streaming output work.
//! The bash executor streaming tests live in `bash_streaming.rs`; here we focus
//! on the decode → extract path that is shared by all protocols.

use earl::protocol::extract::extract_result;
use earl::template::schema::{ResultDecode, ResultExtract};
use earl_core::StreamChunk;
use earl_core::decode::{DecodedBody, decode_response};
use serde_json::json;

// ── JSON chunks ───────────────────────────────────────────

#[test]
fn streaming_json_chunk_is_independently_decodable() {
    let chunk = StreamChunk {
        data: br#"{"msg":"hello"}"#.to_vec(),
        content_type: Some("application/json".to_string()),
    };

    let decoded = decode_response(
        ResultDecode::Json,
        chunk.content_type.as_deref(),
        &chunk.data,
    )
    .unwrap();
    let DecodedBody::Json(v) = decoded else { panic!("expected DecodedBody::Json") };
    assert_eq!(v, json!({"msg": "hello"}));
}

#[test]
fn streaming_json_pointer_extract_returns_value_at_specified_path() {
    let chunk = StreamChunk {
        data: br#"{"data":{"id":1}}"#.to_vec(),
        content_type: Some("application/json".to_string()),
    };

    let extract = ResultExtract::JsonPointer {
        json_pointer: "/data/id".to_string(),
    };

    let decoded = decode_response(ResultDecode::Json, chunk.content_type.as_deref(), &chunk.data).unwrap();
    assert_eq!(extract_result(Some(&extract), &decoded).unwrap(), json!(1));
}

// ── Text / line-oriented chunks ──────────────────────────

#[test]
fn streaming_regex_extract_returns_first_capture_group_from_chunk() {
    let chunk = StreamChunk {
        data: b"event_id=abc-001 status=ok".to_vec(),
        content_type: Some("text/plain".to_string()),
    };

    let extract = ResultExtract::Regex {
        regex: r"event_id=([a-z0-9-]+)".to_string(),
    };

    let decoded = decode_response(ResultDecode::Text, chunk.content_type.as_deref(), &chunk.data).unwrap();
    assert_eq!(extract_result(Some(&extract), &decoded).unwrap(), json!("abc-001"));
}

// ── Auto decode ──────────────────────────────────────────

#[test]
fn streaming_auto_decode_infers_json_from_content_type() {
    let chunk = StreamChunk {
        data: br#"{"ok":true}"#.to_vec(),
        content_type: Some("application/json".to_string()),
    };

    let decoded = decode_response(
        ResultDecode::Auto,
        chunk.content_type.as_deref(),
        &chunk.data,
    )
    .unwrap();
    let DecodedBody::Json(v) = decoded else { panic!("expected DecodedBody::Json") };
    assert_eq!(v, json!({"ok": true}));
}

#[test]
fn streaming_auto_decode_infers_text_from_content_type() {
    let chunk = StreamChunk {
        data: b"plain text line".to_vec(),
        content_type: Some("text/plain".to_string()),
    };

    let decoded = decode_response(
        ResultDecode::Auto,
        chunk.content_type.as_deref(),
        &chunk.data,
    )
    .unwrap();
    let DecodedBody::Text(v) = decoded else { panic!("expected DecodedBody::Text") };
    assert_eq!(v, "plain text line");
}

#[test]
fn streaming_auto_decode_falls_back_to_json_for_valid_json_without_content_type() {
    let chunk = StreamChunk {
        data: br#"{"key":"value"}"#.to_vec(),
        content_type: None,
    };

    let decoded = decode_response(
        ResultDecode::Auto,
        chunk.content_type.as_deref(),
        &chunk.data,
    )
    .unwrap();
    let DecodedBody::Json(v) = decoded else { panic!("expected DecodedBody::Json") };
    assert_eq!(v, json!({"key": "value"}));
}

// ── No-extract passthrough ───────────────────────────────

#[test]
fn streaming_chunk_with_no_extract_returns_full_decoded_value() {
    let chunk = StreamChunk {
        data: br#"{"a":1,"b":"two"}"#.to_vec(),
        content_type: Some("application/json".to_string()),
    };

    let decoded = decode_response(
        ResultDecode::Json,
        chunk.content_type.as_deref(),
        &chunk.data,
    )
    .unwrap();
    let value = extract_result(None, &decoded).unwrap();

    assert_eq!(value, json!({"a": 1, "b": "two"}));
}

// ── HTML / CSS selector chunks ───────────────────────────

#[test]
fn streaming_css_selector_extract_returns_matching_element_text() {
    let chunk = StreamChunk {
        data: b"<html><body><span class=\"val\">100</span></body></html>".to_vec(),
        content_type: Some("text/html".to_string()),
    };

    let extract = ResultExtract::CssSelector {
        css_selector: "span.val".to_string(),
    };

    let decoded = decode_response(ResultDecode::Html, chunk.content_type.as_deref(), &chunk.data).unwrap();
    assert_eq!(extract_result(Some(&extract), &decoded).unwrap(), json!(["100"]));
}

// ── XML / XPath chunks ──────────────────────────────────

#[test]
fn streaming_xpath_extract_returns_text_node_values() {
    let chunk = StreamChunk {
        data: b"<root><item>alpha</item></root>".to_vec(),
        content_type: Some("application/xml".to_string()),
    };

    let extract = ResultExtract::XPath {
        xpath: "//item/text()".to_string(),
    };

    let decoded = decode_response(ResultDecode::Xml, chunk.content_type.as_deref(), &chunk.data).unwrap();
    assert_eq!(extract_result(Some(&extract), &decoded).unwrap(), json!(["alpha"]));
}

// ── Binary chunks ────────────────────────────────────────

#[test]
fn streaming_binary_chunk_extract_returns_base64_encoded_string() {
    let chunk = StreamChunk {
        data: vec![0x00, 0xFF, 0xAB, 0xCD],
        content_type: Some("application/octet-stream".to_string()),
    };

    let decoded = decode_response(
        ResultDecode::Binary,
        chunk.content_type.as_deref(),
        &chunk.data,
    )
    .unwrap();
    let value = extract_result(None, &decoded).unwrap();

    assert_eq!(value, json!("AP+rzQ=="));
}

// ── Error case: malformed JSON in a chunk ────────────────

#[test]
fn streaming_chunk_with_malformed_json_returns_error() {
    let chunk = StreamChunk {
        data: b"not valid json {{{".to_vec(),
        content_type: Some("application/json".to_string()),
    };

    let result = decode_response(
        ResultDecode::Json,
        chunk.content_type.as_deref(),
        &chunk.data,
    );
    assert!(
        result.is_err(),
        "malformed JSON in a chunk should produce an error"
    );
}
