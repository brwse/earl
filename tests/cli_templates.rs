use std::fs;
use std::path::Path;

use assert_cmd::cargo::cargo_bin_cmd;
use httpmock::prelude::*;
use serde_json::Value;

const GITHUB_SAMPLE_TEMPLATE: &str = include_str!("fixtures/templates/github_sample.hcl");

fn write_template(cwd: &std::path::Path) {
    let templates_dir = cwd.join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(templates_dir.join("github.hcl"), GITHUB_SAMPLE_TEMPLATE).unwrap();
}

fn write_config(home: &std::path::Path) {
    let config_dir = home.join(".config/earl");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"
[search]
top_k = 40
rerank_k = 10

[search.local]
embedding_model = "invalid-model"
reranker_model = "invalid-model"

[search.remote]
enabled = false

[[network.allow]]
scheme = "https"
host = "api.github.com"
port = 443
path_prefix = "/"

[[network.allow]]
scheme = "https"
host = "api.example.com"
port = 443
path_prefix = "/"
"#,
    )
    .unwrap();
}

fn write_source_template(cwd: &Path, rel_path: &str, template: &str) -> std::path::PathBuf {
    let source_path = cwd.join(rel_path);
    if let Some(parent) = source_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&source_path, template).unwrap();
    source_path
}

#[test]
fn templates_list_write_mode_filter_shows_write_commands() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("github.create_issue"));
}

#[test]
fn templates_list_write_mode_filter_hides_read_commands() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(!stdout.contains("github.search_issues"));
}

#[test]
fn templates_list_shows_input_schema_section_header() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Input Schema"));
}

#[test]
fn templates_list_write_mode_input_schema_includes_required_field() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("- owner: string (required"));
}

#[test]
fn templates_list_discovers_nested_local_templates() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let templates_dir = cwd.path().join("templates/acme/tools");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(templates_dir.join("github.hcl"), GITHUB_SAMPLE_TEMPLATE).unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "list", "--json"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let json_str = serde_json::to_string(&parsed).unwrap();
    assert!(json_str.contains("github.create_issue"));
}

#[test]
fn templates_generate_shows_wizard_header() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > /dev/null",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Template generation wizard"));
}

#[test]
fn templates_generate_shows_description_prompt() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > /dev/null",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Describe the template you want"));
}

#[test]
fn templates_generate_shows_coding_cli_progress_message() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > /dev/null",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Sending prompt to coding CLI"));
}

#[test]
fn templates_generate_does_not_show_deprecated_command_mode_prompt() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > /dev/null",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(!stdout.contains("Command mode (read/write)"));
}

#[test]
fn templates_generate_does_not_show_deprecated_file_path_prompt() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > /dev/null",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(!stdout.contains("Template file path"));
}

#[test]
fn templates_generate_prompt_includes_user_request_label() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let capture_file = cwd.path().join("prompt.txt");

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .env("EARL_CAPTURE_FILE", &capture_file)
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > \"$EARL_CAPTURE_FILE\"",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    cmd.assert().success();

    let prompt = fs::read_to_string(capture_file).unwrap();
    assert!(prompt.contains("User request:"));
}

#[test]
fn templates_generate_prompt_includes_user_request_text() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let capture_file = cwd.path().join("prompt.txt");

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .env("EARL_CAPTURE_FILE", &capture_file)
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > \"$EARL_CAPTURE_FILE\"",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    cmd.assert().success();

    let prompt = fs::read_to_string(capture_file).unwrap();
    assert!(prompt.contains("Please create github.create_issue"));
}

#[test]
fn templates_generate_prompt_includes_likely_command_key() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let capture_file = cwd.path().join("prompt.txt");

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .env("EARL_CAPTURE_FILE", &capture_file)
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > \"$EARL_CAPTURE_FILE\"",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    cmd.assert().success();

    let prompt = fs::read_to_string(capture_file).unwrap();
    assert!(prompt.contains("- likely command key: `github.create_issue`"));
}

#[test]
fn templates_generate_prompt_includes_likely_file_path() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let capture_file = cwd.path().join("prompt.txt");

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .env("EARL_CAPTURE_FILE", &capture_file)
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > \"$EARL_CAPTURE_FILE\"",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    cmd.assert().success();

    let prompt = fs::read_to_string(capture_file).unwrap();
    assert!(prompt.contains("- likely file: `templates/github.hcl`"));
}

#[test]
fn templates_generate_prompt_includes_validation_hint() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let capture_file = cwd.path().join("prompt.txt");

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .env("EARL_CAPTURE_FILE", &capture_file)
        .args([
            "templates",
            "generate",
            "--",
            "sh",
            "-c",
            "cat > \"$EARL_CAPTURE_FILE\"",
        ])
        .write_stdin(
            "Please create github.create_issue to open a GitHub issue using owner/repo/title/body and github.token.\n",
        );

    cmd.assert().success();

    let prompt = fs::read_to_string(capture_file).unwrap();
    assert!(prompt.contains("Run `earl templates validate`"));
}

#[test]
fn templates_generate_rejects_json_mode() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "--json",
        "generate",
        "--",
        "sh",
        "-c",
        "cat",
    ]);

    let out = cmd.assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("does not support --json"));
}

#[test]
fn templates_import_rejects_unsupported_url_scheme() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        "git://github.com/mathematic-inc/earl/templates/github.hcl",
    ]);

    let out = cmd.assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("unsupported template URL scheme"));
}

#[test]
fn templates_import_from_local_path_shows_success_message() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Imported template"));
}

#[test]
fn templates_import_with_no_required_secrets_reports_none_declared() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("No required secrets were declared"));
}

#[test]
fn templates_import_from_local_path_writes_source_file_contents() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    cmd.assert().success();

    let imported_path = cwd.path().join("templates/github.hcl");
    let imported = fs::read_to_string(imported_path).unwrap();
    assert!(imported.contains("provider"));
    assert!(imported.contains("\"demo\""));
    assert!(imported.contains("command \"ping\""));
}

#[test]
fn templates_import_with_global_scope_stores_file_in_global_config_dir() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--scope",
        "global",
    ]);

    cmd.assert().success();

    let imported_path = home.path().join(".config/earl/templates/github.hcl");
    assert!(imported_path.exists());
}

#[test]
fn templates_import_with_global_scope_writes_correct_file_content() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--scope",
        "global",
    ]);

    cmd.assert().success();

    let imported_path = home.path().join(".config/earl/templates/github.hcl");
    let imported = fs::read_to_string(imported_path).unwrap();
    assert!(imported.contains("provider"));
    assert!(imported.contains("\"demo\""));
    assert!(imported.contains("command \"ping\""));
}

#[test]
fn templates_import_from_http_url_shows_success_message() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let server = MockServer::start();
    let template = include_str!("fixtures/templates/valid_minimal.hcl");
    server.mock(|when, then| {
        when.method(GET).path("/github.hcl");
        then.status(200).body(template);
    });
    let source_url = format!("{}/github.hcl", server.base_url());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_url.as_str(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Imported template"));
}

#[test]
fn templates_import_from_http_url_shows_destination_path_in_output() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let server = MockServer::start();
    let template = include_str!("fixtures/templates/valid_minimal.hcl");
    server.mock(|when, then| {
        when.method(GET).path("/github.hcl");
        then.status(200).body(template);
    });
    let source_url = format!("{}/github.hcl", server.base_url());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_url.as_str(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("templates/github.hcl"));
}

#[test]
fn templates_import_from_http_url_requests_the_template_url() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let server = MockServer::start();
    let template = include_str!("fixtures/templates/valid_minimal.hcl");
    let template_mock = server.mock(|when, then| {
        when.method(GET).path("/github.hcl");
        then.status(200).body(template);
    });
    let source_url = format!("{}/github.hcl", server.base_url());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_url.as_str(),
    ]);

    cmd.assert().success();
    template_mock.assert();
}

#[test]
fn templates_import_fails_when_local_source_is_missing() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        "missing/github.hcl",
    ]);

    let out = cmd.assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("was not found or is not a file"));
}

#[test]
fn templates_import_shows_required_secrets_section_header() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Required secrets:"));
}

#[test]
fn templates_import_lists_required_secret_names_in_output() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("- github.token"));
}

#[test]
fn templates_import_shows_secret_setup_section_header() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Set up with:"));
}

#[test]
fn templates_import_shows_secrets_set_command_in_setup_instructions() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("earl secrets set github.token"));
}

#[test]
fn templates_import_json_output_source_ref_reflects_input_path() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        parsed["source_ref"],
        source_path.to_string_lossy().to_string()
    );
}

#[test]
fn templates_import_json_output_source_reflects_input_path() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(parsed["source"], source_path.to_string_lossy().to_string());
}

#[test]
fn templates_import_json_output_lists_required_secrets() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        parsed["required_secrets"].as_array().unwrap(),
        &vec![Value::String("github.token".to_string())]
    );
}

#[test]
fn templates_import_json_output_destination_path_uses_filename() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path =
        write_source_template(cwd.path(), "source/github.hcl", GITHUB_SAMPLE_TEMPLATE);

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let destination = parsed["destination"].as_str().unwrap();
    assert!(Path::new(destination).ends_with(Path::new("templates/github.hcl")));
}

#[test]
fn templates_import_json_global_scope_reports_global_destination() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
        "--scope",
        "global",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let destination = parsed["destination"].as_str().unwrap();
    let expected = home.path().join(".config/earl/templates/github.hcl");
    assert_eq!(Path::new(destination), expected.as_path());
}

#[test]
fn templates_import_refuses_to_overwrite_existing_template() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(cwd.path().join("templates")).unwrap();
    fs::write(cwd.path().join("templates/github.hcl"), "version = 1\n").unwrap();
    let source_path = write_source_template(
        cwd.path(),
        "source/github.hcl",
        include_str!("fixtures/templates/valid_minimal.hcl"),
    );

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("already exists"));
}

#[test]
fn templates_import_rejects_non_hcl_file_extension() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let source_path = write_source_template(cwd.path(), "source/github.json", "version = 1\n");

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        source_path.to_str().unwrap(),
    ]);

    let out = cmd.assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("template file must end with .hcl"));
}

#[test]
fn templates_list_works_with_empty_global_allowlist() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "list"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("github.create_issue"));
}

#[test]
fn templates_list_json_write_mode_output_includes_expected_command() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let rows = parsed.as_array().unwrap();
    assert!(!rows.is_empty());
    assert_eq!(
        rows[0]["command"], "github.create_issue",
        "github.create_issue should be present in write-mode listings"
    );
}

#[test]
fn templates_list_json_write_mode_output_has_write_mode_field() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let rows = parsed.as_array().unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0]["mode"], "write");
}

#[test]
fn templates_list_json_write_mode_output_has_local_scope() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let rows = parsed.as_array().unwrap();
    assert!(!rows.is_empty());
    assert_eq!(rows[0]["source"]["scope"], "local");
}

#[test]
fn templates_list_json_output_includes_input_schema() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "list",
        "--mode",
        "write",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let rows = parsed.as_array().unwrap();
    assert!(!rows.is_empty());
    let create_issue = &rows[0];
    let schema = serde_json::to_string(&create_issue["input_schema"]).unwrap();
    assert!(schema.contains(r#""owner""#));
}

#[test]
fn templates_validate_succeeds_with_valid_template() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let templates_dir = cwd.path().join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("ok.hcl"),
        include_str!("fixtures/templates/valid_minimal.hcl"),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "validate"]);
    cmd.assert().success();
}

#[test]
fn templates_validate_fails_with_invalid_template() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let templates_dir = cwd.path().join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("bad.hcl"),
        include_str!("fixtures/templates/invalid_secret_ref.hcl"),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "validate"]);
    cmd.assert().failure();
}

#[test]
fn templates_validate_supports_json_output() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let templates_dir = cwd.path().join("templates");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("ok.hcl"),
        include_str!("fixtures/templates/valid_minimal.hcl"),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "validate", "--json"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let files = parsed.as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].as_str().unwrap().ends_with("ok.hcl"));
}

#[test]
fn templates_validate_supports_nested_template_paths() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    write_config(home.path());

    let templates_dir = cwd.path().join("templates/mathematic-inc/core");
    fs::create_dir_all(&templates_dir).unwrap();
    fs::write(
        templates_dir.join("ok.hcl"),
        include_str!("fixtures/templates/valid_minimal.hcl"),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "validate", "--json"]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let files = parsed.as_array().unwrap();
    assert_eq!(files.len(), 1);
    let validated_path = std::path::Path::new(files[0].as_str().unwrap());
    let expected_suffix = std::path::Path::new("templates")
        .join("mathematic-inc")
        .join("core")
        .join("ok.hcl");
    assert!(validated_path.ends_with(expected_suffix));
}

#[test]
fn templates_search_fallback_shows_command_name() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("github.create_issue"));
}

#[test]
fn templates_search_fallback_shows_summary_label() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Summary"));
}

#[test]
fn templates_search_fallback_hides_description_field() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(!stdout.contains("Description"));
}

#[test]
fn templates_search_fallback_hides_input_schema_field() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(!stdout.contains("Input Schema"));
}

#[test]
fn templates_search_fallback_hides_agent_guidance_field() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(!stdout.contains("Guidance for AI agents"));
}

#[test]
fn templates_search_json_output_includes_matching_command() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let hits = parsed.as_array().unwrap();
    assert!(!hits.is_empty());
    let json_str = serde_json::to_string(&parsed).unwrap();
    assert!(json_str.contains("github.create_issue"));
}

#[test]
fn templates_search_json_output_results_include_score_field() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_template(cwd.path());
    write_config(home.path());

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "search",
        "Bug: login fails",
        "--limit",
        "5",
        "--json",
    ]);

    let out = cmd.assert().success().get_output().stdout.clone();
    let parsed: Value = serde_json::from_slice(&out).unwrap();
    let hits = parsed.as_array().unwrap();
    assert!(!hits.is_empty());
    assert!(hits[0]["score"].as_f64().is_some());
}
