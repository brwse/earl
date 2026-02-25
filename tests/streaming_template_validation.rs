//! Integration tests verifying that templates with `stream = true` are accepted
//! by the template validator, and that the `is_streaming()` helper returns the
//! correct value for each protocol variant.

use std::fs;
use std::path::Path;

use earl::template::loader::validate_all_from_dirs;
use earl::template::parser::parse_template_hcl;
use tempfile::tempdir;

#[test]
#[cfg(feature = "http")]
fn http_template_with_stream_true_is_valid() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "events" {
  title = "Events"
  summary = "Stream events from server"
  description = "Opens a streaming connection and prints events as they arrive."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.example.com/events"
    stream = true
  }

  result {
    decode = "json"
    output = "{{ result }}"
  }
}
"#;
    fs::write(local_dir.join("stream_http.hcl"), hcl).unwrap();

    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1, "stream=true HTTP template should be valid");
}

#[test]
#[cfg(feature = "http")]
fn http_template_with_stream_false_is_valid() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch a resource"
  description = "Standard non-streaming HTTP fetch."

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
    fs::write(local_dir.join("no_stream.hcl"), hcl).unwrap();

    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1, "stream=false HTTP template should be valid");
}

#[test]
#[cfg(feature = "bash")]
fn bash_template_with_stream_true_is_valid() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "tail" {
  title = "Tail"
  summary = "Tail a log file"
  description = "Streams output from a bash script line by line."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "bash"
    stream = true

    bash {
      script = "echo streaming"
    }
  }

  result {
    output = "{{ result }}"
  }
}
"#;
    fs::write(local_dir.join("stream_bash.hcl"), hcl).unwrap();

    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1, "stream=true bash template should be valid");
}

#[test]
#[cfg(feature = "grpc")]
fn grpc_template_with_stream_true_is_valid() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "watch" {
  title = "Watch"
  summary = "Watch gRPC server stream"
  description = "Opens a server-streaming gRPC call."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "grpc"
    url = "http://localhost:50051"
    stream = true

    grpc {
      service = "example.Watcher"
      method = "Watch"
      body = {}
    }
  }

  result {
    output = "{{ result }}"
  }
}
"#;
    fs::write(local_dir.join("stream_grpc.hcl"), hcl).unwrap();

    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1, "stream=true gRPC template should be valid");
}

// ── is_streaming() helper tests ─────────────────────────

#[test]
#[cfg(feature = "http")]
fn http_operation_with_stream_true_is_streaming() {
    let hcl_src = r#"
version = 1
provider = "demo"

command "events" {
  title = "Events"
  summary = "Stream events"
  description = "Streams events."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.example.com/events"
    stream = true
  }

  result {
    output = "{{ result }}"
  }
}
"#;
    let template_file = parse_template_hcl(hcl_src, Path::new(".")).unwrap();

    let cmd = template_file.commands.get("events").unwrap();
    assert!(
        cmd.operation.is_streaming(),
        "is_streaming() should return true when stream = true"
    );
}

#[test]
#[cfg(feature = "http")]
fn http_operation_without_stream_field_is_not_streaming() {
    let hcl_src = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Normal fetch"
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
        "is_streaming() should return false by default"
    );
}
