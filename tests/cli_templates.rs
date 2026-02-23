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
fn templates_list_filters_mode_and_category() {
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
    assert!(!stdout.contains("github.search_issues"));
    assert!(stdout.contains("Input Schema"));
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
    let rows = parsed.as_array().unwrap();
    assert!(!rows.is_empty());
    assert!(
        rows.iter()
            .any(|row| row["command"] == "github.create_issue")
    );
}

#[test]
fn templates_import_rejects_unsupported_url_scheme() {
    let cwd = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let mut cmd = cargo_bin_cmd!("earl");
    cmd.current_dir(cwd.path()).env("HOME", home.path()).args([
        "templates",
        "import",
        "git://github.com/brwse/earl-core/templates/github.hcl",
    ]);

    let out = cmd.assert().failure().get_output().stderr.clone();
    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("unsupported template URL scheme"));
}

#[test]
fn templates_import_from_local_path_imports_template() {
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
    assert!(stdout.contains("No required secrets were declared"));

    let imported_path = cwd.path().join("templates/github.hcl");
    let imported = fs::read_to_string(imported_path).unwrap();
    assert!(imported.contains("provider"));
    assert!(imported.contains("\"demo\""));
    assert!(imported.contains("command \"ping\""));
}

#[test]
fn templates_import_with_global_scope_imports_template() {
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

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Imported template"));

    let imported_path = home.path().join(".config/earl/templates/github.hcl");
    assert!(imported_path.exists());
    let imported = fs::read_to_string(imported_path).unwrap();
    assert!(imported.contains("provider"));
    assert!(imported.contains("\"demo\""));
    assert!(imported.contains("command \"ping\""));
}

#[test]
fn templates_import_from_http_url_imports_template() {
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

    let out = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Imported template"));
    assert!(stdout.contains("templates/github.hcl"));
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
fn templates_import_reports_required_secrets_to_user() {
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
    assert!(stdout.contains("- github.token"));
    assert!(stdout.contains("Set up with:"));
    assert!(stdout.contains("earl secrets set github.token"));
}

#[test]
fn templates_import_json_includes_required_secrets() {
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
    assert_eq!(parsed["source"], source_path.to_string_lossy().to_string());
    assert_eq!(
        parsed["required_secrets"].as_array().unwrap(),
        &vec![Value::String("github.token".to_string())]
    );
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
    assert!(stdout.contains("- owner: string (required"));
}

#[test]
fn templates_list_supports_json_output() {
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
    let create_issue = rows
        .iter()
        .find(|row| row["command"] == "github.create_issue")
        .expect("github.create_issue should be present in write-mode listings");
    assert_eq!(create_issue["mode"], "write");
    assert_eq!(create_issue["source"]["scope"], "local");
    assert!(
        create_issue["input_schema"]
            .as_array()
            .unwrap()
            .iter()
            .any(|param| param["name"] == "owner")
    );
}

#[test]
fn templates_validate_reports_success_and_failure() {
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

    let mut ok_cmd = cargo_bin_cmd!("earl");
    ok_cmd
        .current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "validate"]);
    ok_cmd.assert().success();

    fs::write(
        templates_dir.join("bad.hcl"),
        include_str!("fixtures/templates/invalid_secret_ref.hcl"),
    )
    .unwrap();

    let mut bad_cmd = cargo_bin_cmd!("earl");
    bad_cmd
        .current_dir(cwd.path())
        .env("HOME", home.path())
        .args(["templates", "validate"]);

    bad_cmd.assert().failure();
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

    let templates_dir = cwd.path().join("templates/brwse/core");
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
        .join("brwse")
        .join("core")
        .join("ok.hcl");
    assert!(validated_path.ends_with(expected_suffix));
}

#[test]
fn templates_search_uses_deterministic_fallback() {
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
    assert!(stdout.contains("Summary"));
    assert!(!stdout.contains("Description"));
    assert!(!stdout.contains("Input Schema"));
    assert!(!stdout.contains("Guidance for AI agents"));
}

#[test]
fn templates_search_supports_json_output() {
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
    assert!(hits.iter().any(|hit| hit["key"] == "github.create_issue"));
    assert!(hits[0]["score"].as_f64().is_some());
}
