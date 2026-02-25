//! Tests for the streaming output pipeline and backwards compatibility.

use std::path::Path;

use earl::output::stream::render_streaming_output;
use earl::template::parser::parse_template_hcl;
use earl::template::schema::{ResultDecode, ResultTemplate};
use earl_core::{Redactor, StreamChunk};
use serde_json::Map;
use tokio::sync::mpsc;

// ── render_streaming_output tests ───────────────────────────

#[tokio::test]
async fn json_chunks_are_processed_without_error() {
    let (tx, rx) = mpsc::channel::<StreamChunk>(16);

    tokio::spawn(async move {
        tx.send(StreamChunk {
            data: br#"{"msg":"hello"}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        })
        .await
        .unwrap();
        tx.send(StreamChunk {
            data: br#"{"msg":"world"}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        })
        .await
        .unwrap();
    });

    let result_template = ResultTemplate::default();
    let args = Map::new();
    let redactor = Redactor::new(vec![]);

    let result = render_streaming_output(rx, &result_template, &args, &redactor, true, None).await;
    assert!(result.is_ok(), "should process JSON chunks without error");
}

#[tokio::test]
async fn malformed_json_chunk_is_skipped_and_processing_continues() {
    let (tx, rx) = mpsc::channel::<StreamChunk>(16);

    tokio::spawn(async move {
        tx.send(StreamChunk {
            data: b"not valid json".to_vec(),
            content_type: Some("application/json".to_string()),
        })
        .await
        .unwrap();
        tx.send(StreamChunk {
            data: br#"{"ok":true}"#.to_vec(),
            content_type: Some("application/json".to_string()),
        })
        .await
        .unwrap();
    });

    let result_template = ResultTemplate {
        decode: ResultDecode::Json,
        extract: None,
        output: "{{ result }}".to_string(),
        result_alias: None,
    };
    let args = Map::new();
    let redactor = Redactor::new(vec![]);

    let result = render_streaming_output(rx, &result_template, &args, &redactor, true, None).await;
    assert!(result.is_ok(), "should skip bad chunks and continue");
}

#[tokio::test]
async fn empty_channel_returns_ok() {
    let (_tx, rx) = mpsc::channel::<StreamChunk>(1);
    drop(_tx);

    let result_template = ResultTemplate::default();
    let args = Map::new();
    let redactor = Redactor::new(vec![]);

    let result = render_streaming_output(rx, &result_template, &args, &redactor, false, None).await;
    assert!(result.is_ok(), "empty channel should return Ok");
}

// ── Backwards compatibility tests ───────────────────────────

#[test]
#[cfg(feature = "http")]
fn non_streaming_template_defaults_to_false() {
    let hcl_src = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch data"
  description = "Non-streaming."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.example.com/data"
  }

  result {
    output = "{{ result }}"
  }
}
"#;
    let template_file = parse_template_hcl(hcl_src, Path::new(".")).unwrap();
    let cmd = template_file.commands.get("fetch").unwrap();
    assert!(
        !cmd.operation.is_streaming(),
        "should default to false when stream field is absent"
    );
}

#[test]
#[cfg(feature = "http")]
fn template_with_stream_false_returns_false() {
    let hcl_src = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch data"
  description = "Explicitly non-streaming."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.example.com/data"
    stream = false
  }

  result {
    output = "{{ result }}"
  }
}
"#;
    let template_file = parse_template_hcl(hcl_src, Path::new(".")).unwrap();
    let cmd = template_file.commands.get("fetch").unwrap();
    assert!(
        !cmd.operation.is_streaming(),
        "should return false when stream = false is explicit"
    );
}
