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
fn streaming_json_chunks_are_independently_decodable() {
    let chunks = [
        StreamChunk {
            data: br#"{"msg":"hello"}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        },
        StreamChunk {
            data: br#"{"msg":"world"}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        },
    ];

    for (i, chunk) in chunks.iter().enumerate() {
        let decoded = decode_response(
            ResultDecode::Json,
            chunk.content_type.as_deref(),
            &chunk.data,
        );
        assert!(
            decoded.is_ok(),
            "chunk {i} should be independently decodable: {:?}",
            decoded.err()
        );
    }
}

#[test]
fn streaming_json_chunks_with_extract_json_pointer() {
    let chunks = [
        StreamChunk {
            data: br#"{"data":{"id":1}}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        },
        StreamChunk {
            data: br#"{"data":{"id":2}}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        },
        StreamChunk {
            data: br#"{"data":{"id":3}}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        },
    ];

    let extract = ResultExtract::JsonPointer {
        json_pointer: "/data/id".to_string(),
    };

    let mut extracted_ids = vec![];
    for chunk in &chunks {
        let decoded = decode_response(
            ResultDecode::Json,
            chunk.content_type.as_deref(),
            &chunk.data,
        )
        .unwrap();
        let value = extract_result(Some(&extract), &decoded).unwrap();
        extracted_ids.push(value);
    }

    assert_eq!(extracted_ids, vec![json!(1), json!(2), json!(3)]);
}

// ── Text / line-oriented chunks ──────────────────────────

#[test]
fn streaming_text_chunks_with_regex_extract() {
    let chunks = [
        StreamChunk {
            data: b"event_id=abc-001 status=ok".to_vec(),
            content_type: Some("text/plain".to_string()),
        },
        StreamChunk {
            data: b"event_id=def-002 status=error".to_vec(),
            content_type: Some("text/plain".to_string()),
        },
    ];

    let extract = ResultExtract::Regex {
        regex: r"event_id=([a-z0-9-]+)".to_string(),
    };

    let mut event_ids = vec![];
    for chunk in &chunks {
        let decoded = decode_response(
            ResultDecode::Text,
            chunk.content_type.as_deref(),
            &chunk.data,
        )
        .unwrap();
        let value = extract_result(Some(&extract), &decoded).unwrap();
        event_ids.push(value);
    }

    assert_eq!(event_ids, vec![json!("abc-001"), json!("def-002")]);
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
    match decoded {
        DecodedBody::Json(v) => assert_eq!(v, json!({"ok": true})),
        other => panic!("expected Json, got {other:?}"),
    }
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
    match decoded {
        DecodedBody::Text(v) => assert_eq!(v, "plain text line"),
        other => panic!("expected Text, got {other:?}"),
    }
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
    match decoded {
        DecodedBody::Json(v) => assert_eq!(v, json!({"key": "value"})),
        other => panic!("expected Json, got {other:?}"),
    }
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
fn streaming_html_chunks_with_css_selector_extract() {
    let chunks = [
        StreamChunk {
            data: b"<html><body><span class=\"val\">100</span></body></html>".to_vec(),
            content_type: Some("text/html".to_string()),
        },
        StreamChunk {
            data: b"<html><body><span class=\"val\">200</span></body></html>".to_vec(),
            content_type: Some("text/html".to_string()),
        },
    ];

    let extract = ResultExtract::CssSelector {
        css_selector: "span.val".to_string(),
    };

    let mut values = vec![];
    for chunk in &chunks {
        let decoded = decode_response(
            ResultDecode::Html,
            chunk.content_type.as_deref(),
            &chunk.data,
        )
        .unwrap();
        let value = extract_result(Some(&extract), &decoded).unwrap();
        values.push(value);
    }

    assert_eq!(values, vec![json!(["100"]), json!(["200"])]);
}

// ── XML / XPath chunks ──────────────────────────────────

#[test]
fn streaming_xml_chunks_with_xpath_extract() {
    let chunks = [
        StreamChunk {
            data: b"<root><item>alpha</item></root>".to_vec(),
            content_type: Some("application/xml".to_string()),
        },
        StreamChunk {
            data: b"<root><item>beta</item></root>".to_vec(),
            content_type: Some("application/xml".to_string()),
        },
    ];

    let extract = ResultExtract::XPath {
        xpath: "//item/text()".to_string(),
    };

    let mut values = vec![];
    for chunk in &chunks {
        let decoded = decode_response(
            ResultDecode::Xml,
            chunk.content_type.as_deref(),
            &chunk.data,
        )
        .unwrap();
        let value = extract_result(Some(&extract), &decoded).unwrap();
        values.push(value);
    }

    assert_eq!(values, vec![json!(["alpha"]), json!(["beta"])]);
}

// ── Binary chunks ────────────────────────────────────────

#[test]
fn streaming_binary_chunks_decode_without_error() {
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

    // Binary data is base64 encoded when converted to JSON.
    assert!(value.is_string(), "binary should be base64-encoded string");
    assert!(!value.as_str().unwrap().is_empty());
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
