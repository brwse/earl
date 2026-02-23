use anyhow::Result;
use serde_json::{Map, Value};
use tokio::sync::mpsc;

use earl_core::decode::DecodedBody;
use earl_core::schema::ResultTemplate;
use earl_core::{Redactor, StreamChunk, decode_response};

use crate::output::human::render_human_output;
use crate::protocol::extract::extract_result;

/// Consume streaming chunks and render each one to stdout.
///
/// Each [`StreamChunk`] is decoded, extracted, and printed independently,
/// giving the user real-time output as data arrives from the server.
pub async fn render_streaming_output(
    mut rx: mpsc::Receiver<StreamChunk>,
    result_template: &ResultTemplate,
    args: &Map<String, Value>,
    redactor: &Redactor,
    json_mode: bool,
) -> Result<()> {
    while let Some(chunk) = rx.recv().await {
        let decoded_body: DecodedBody = decode_response(
            result_template.decode,
            chunk.content_type.as_deref(),
            &chunk.data,
        )?;
        let extracted = extract_result(result_template.extract.as_ref(), &decoded_body)?;

        if json_mode {
            let obj = Value::Object(Map::from_iter([
                ("result".to_string(), extracted.clone()),
                ("decoded".to_string(), decoded_body.to_json_value()),
            ]));
            let redacted = redactor.redact_json(&obj);
            println!("{}", serde_json::to_string(&redacted)?);
        } else {
            let output = render_human_output(result_template, args, &extracted)?;
            println!("{}", redactor.redact(&output));
        }
    }

    Ok(())
}
