use earl_core::Redactor;
use serde_json::json;

#[test]
fn plaintext_secret_removed_from_output() {
    let redactor = Redactor::new(vec!["token-abc".to_string()]);
    let input = "Authorization: Bearer token-abc";
    let output = redactor.redact(input);
    assert!(!output.contains("token-abc"));
}

#[test]
fn plaintext_secret_replaced_with_redacted_marker() {
    let redactor = Redactor::new(vec!["token-abc".to_string()]);
    let input = "Authorization: Bearer token-abc";
    let output = redactor.redact(input);
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn overlapping_secrets_removed_from_output() {
    let redactor = Redactor::new(vec!["token-abc".to_string(), "abc".to_string()]);
    let input = "Authorization: Bearer token-abc";
    let output = redactor.redact(input);
    assert!(!output.contains("token-abc"));
}

#[test]
fn json_top_level_value_replaced_with_redacted_marker() {
    let redactor = Redactor::new(vec!["super-secret".to_string()]);
    let payload = json!({
        "token": "super-secret",
        "nested": {
            "arr": ["ok", "super-secret"]
        }
    });

    let redacted = redactor.redact_json(&payload);
    assert_eq!(redacted["token"], json!("[REDACTED]"));
}

#[test]
fn json_nested_array_value_replaced_with_redacted_marker() {
    let redactor = Redactor::new(vec!["super-secret".to_string()]);
    let payload = json!({
        "token": "super-secret",
        "nested": {
            "arr": ["ok", "super-secret"]
        }
    });

    let redacted = redactor.redact_json(&payload);
    assert_eq!(redacted["nested"]["arr"][1], json!("[REDACTED]"));
}

#[test]
fn base64_encoded_secret_removed_from_output() {
    let redactor = Redactor::new(vec!["super-secret".to_string()]);
    let input = "b64=c3VwZXItc2VjcmV0";
    let output = redactor.redact(input);
    assert!(!output.contains("c3VwZXItc2VjcmV0"));
}

#[test]
fn base64_encoded_secret_replaced_with_redacted_marker() {
    let redactor = Redactor::new(vec!["super-secret".to_string()]);
    let input = "b64=c3VwZXItc2VjcmV0";
    let output = redactor.redact(input);
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn hex_encoded_secret_removed_from_output() {
    let redactor = Redactor::new(vec!["super-secret".to_string()]);
    let input = "hex=73757065722d736563726574";
    let output = redactor.redact(input);
    assert!(!output.contains("73757065722d736563726574"));
}

#[test]
fn hex_encoded_secret_replaced_with_redacted_marker() {
    let redactor = Redactor::new(vec!["super-secret".to_string()]);
    let input = "hex=73757065722d736563726574";
    let output = redactor.redact(input);
    assert!(output.contains("[REDACTED]"));
}
