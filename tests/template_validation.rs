use std::{fs, path::Path};

use earl::template::loader::validate_all_from_dirs;
use tempfile::tempdir;

#[test]
#[cfg(feature = "http")]
fn validates_template_files() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    fs::write(
        local_dir.join("valid.hcl"),
        include_str!("fixtures/templates/valid_minimal.hcl"),
    )
    .unwrap();

    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("valid.hcl"));
}

#[test]
#[cfg(feature = "http")]
fn allows_empty_allowlist_rule_set() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    fs::write(
        local_dir.join("invalid.hcl"),
        include_str!("fixtures/templates/invalid_missing_allow.hcl"),
    )
    .unwrap();

    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("invalid.hcl"));
}

#[test]
#[cfg(feature = "http")]
fn fails_on_undeclared_auth_secret_reference() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    fs::write(
        local_dir.join("invalid_secret.hcl"),
        include_str!("fixtures/templates/invalid_secret_ref.hcl"),
    )
    .unwrap();

    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("is not declared in annotations.secrets"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "http")]
fn fails_on_invalid_multipart_part_definition() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "upload" {
  title = "Upload"
  summary = "Upload multipart payload"
  description = "Uploads multipart content to the API."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "POST"
    url = "https://api.example.com/upload"

    body {
      kind = "multipart"
      parts = [
        {
          name = "payload"
          value = "hello"
          file_path = "/tmp/a"
        }
      ]
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("invalid_multipart.hcl"), hcl).unwrap();

    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("must specify exactly one of value, bytes_base64, file_path"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "graphql")]
fn fails_when_graphql_protocol_missing_graphql_block() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "query" {
  title = "Query"
  summary = "Run GraphQL query"
  description = "Runs a GraphQL query against the API."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "graphql"
    method = "POST"
    url = "https://api.example.com/graphql"
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("invalid_graphql.hcl"), hcl).unwrap();

    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("missing field `graphql`"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "grpc")]
fn fails_when_grpc_protocol_missing_grpc_block() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "check" {
  title = "Check"
  summary = "Run gRPC health check"
  description = "Calls a gRPC endpoint."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "grpc"
    url = "http://127.0.0.1:50051"
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("invalid_grpc.hcl"), hcl).unwrap();

    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("missing field `grpc`"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "grpc")]
fn fails_when_grpc_auth_api_key_uses_query_location() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "check" {
  title = "Check"
  summary = "Run gRPC health check"
  description = "Calls a gRPC endpoint."

  annotations {
    mode = "read"
    secrets = ["api.key"]
  }

  operation {
    protocol = "grpc"
    url = "http://127.0.0.1:50051"

    auth {
      kind = "api_key"
      location = "query"
      name = "token"
      secret = "api.key"
    }

    grpc {
      service = "grpc.health.v1.Health"
      method = "Check"
      body = { service = "" }
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("invalid_grpc_api_key.hcl"), hcl).unwrap();

    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("grpc auth api_key location must be `header`"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "grpc")]
fn fails_when_grpc_uses_unsupported_proxy_profile() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "check" {
  title = "Check"
  summary = "Run gRPC health check"
  description = "Calls a gRPC endpoint."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "grpc"
    url = "http://127.0.0.1:50051"

    grpc {
      service = "grpc.health.v1.Health"
      method = "Check"
      body = { service = "" }
    }

    transport {
      proxy_profile = "corp"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("invalid_grpc_proxy.hcl"), hcl).unwrap();

    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("grpc transport.proxy_profile is not supported"),
        "unexpected error: {rendered}"
    );
}

// ── Bash validation tests ────────────────────────────────

#[test]
#[cfg(feature = "bash")]
fn bash_rejects_empty_script() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Positive case: valid bash template
    let valid_hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo hello"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("valid_bash.hcl"), valid_hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);

    // Negative case: empty script
    let invalid_hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "  "
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("valid_bash.hcl"), invalid_hcl).unwrap();
    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("has empty operation.bash.script"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "bash")]
fn bash_rejects_absolute_writable_path() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Positive case: relative writable path
    let valid_hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo hello > out.txt"
      sandbox {
        writable_paths = ["tmp/output"]
      }
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("bash.hcl"), valid_hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);

    // Negative case: absolute path
    let invalid_hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo hello > out.txt"
      sandbox {
        writable_paths = ["/tmp/output"]
      }
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("bash.hcl"), invalid_hcl).unwrap();
    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("contains absolute path"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "bash")]
fn bash_rejects_dotdot_writable_path() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Positive case: path without ..
    let valid_hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo hello > out.txt"
      sandbox {
        writable_paths = ["data/output"]
      }
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("bash.hcl"), valid_hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);

    // Negative case: path with ..
    let invalid_hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo hello > out.txt"
      sandbox {
        writable_paths = ["data/../etc"]
      }
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("bash.hcl"), invalid_hcl).unwrap();
    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("contains `..` in path"),
        "unexpected error: {rendered}"
    );
}

/// Test that paths containing `..` as part of a filename (e.g. `foo..bar`) are allowed.
#[test]
#[cfg(feature = "bash")]
fn bash_allows_dotdot_in_filename() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    let hcl = r#"
version = 1
provider = "demo"

command "run" {
  title = "Run"
  summary = "Run a bash script"
  description = "Executes a bash script in a sandbox."

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo hello > out.txt"
      sandbox {
        writable_paths = ["foo..bar", "data..output"]
      }
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("bash.hcl"), hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(
        files.len(),
        1,
        "paths with `..` in filenames should be allowed"
    );
}

// ── SQL validation tests ─────────────────────────────────

#[test]
#[cfg(feature = "sql")]
fn sql_rejects_empty_query() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Positive case: valid SQL template
    let valid_hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch rows from the database"
  description = "Runs a SQL query against the configured database."

  annotations {
    mode = "read"
    secrets = ["db.url"]
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "db.url"
      query = "SELECT 1"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("sql.hcl"), valid_hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);

    // Negative case: empty query
    let invalid_hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch rows from the database"
  description = "Runs a SQL query against the configured database."

  annotations {
    mode = "read"
    secrets = ["db.url"]
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "db.url"
      query = "  "
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("sql.hcl"), invalid_hcl).unwrap();
    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("has empty operation.sql.query"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "sql")]
fn sql_rejects_jinja_in_query() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Positive case: query without Jinja2 expressions (uses $1 placeholders)
    let valid_hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch rows"
  description = "Runs a SQL query."

  annotations {
    mode = "read"
    secrets = ["db.url"]
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "db.url"
      query = "SELECT * FROM users WHERE id = $1"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("sql.hcl"), valid_hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);

    // Negative case: query with {{ }}
    let invalid_hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch rows"
  description = "Runs a SQL query."

  annotations {
    mode = "read"
    secrets = ["db.url"]
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "db.url"
      query = "SELECT * FROM users WHERE id = {{ args.id }}"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("sql.hcl"), invalid_hcl).unwrap();
    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("must not contain Jinja2 template expressions"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "sql")]
fn sql_rejects_undeclared_connection_secret() {
    let dir = tempdir().unwrap();
    let local_dir = dir.path().join("local");
    let global_dir = dir.path().join("global");
    fs::create_dir_all(&local_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Positive case: connection_secret declared in annotations.secrets
    let valid_hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch rows"
  description = "Runs a SQL query."

  annotations {
    mode = "read"
    secrets = ["db.url"]
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "db.url"
      query = "SELECT 1"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("sql.hcl"), valid_hcl).unwrap();
    let files = validate_all_from_dirs(&global_dir, &local_dir).unwrap();
    assert_eq!(files.len(), 1);

    // Negative case: connection_secret NOT in annotations.secrets
    let invalid_hcl = r#"
version = 1
provider = "demo"

command "fetch" {
  title = "Fetch"
  summary = "Fetch rows"
  description = "Runs a SQL query."

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "sql"

    sql {
      connection_secret = "db.url"
      query = "SELECT 1"
    }
  }

  result {
    output = "ok"
  }
}
"#;
    fs::write(local_dir.join("sql.hcl"), invalid_hcl).unwrap();
    let err = validate_all_from_dirs(&global_dir, &local_dir).unwrap_err();
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("is not declared in annotations.secrets"),
        "unexpected error: {rendered}"
    );
}

#[test]
#[cfg(feature = "http")]
fn validates_all_example_templates() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let examples_dir = manifest_dir.join("examples");
    let empty_dir = tempdir().unwrap();

    let files = validate_all_from_dirs(empty_dir.path(), &examples_dir)
        .expect("example templates should all be valid");

    assert!(
        !files.is_empty(),
        "no .hcl files found in examples/ directory"
    );
}
