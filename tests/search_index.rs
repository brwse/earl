mod common;

use earl::search::index::{SearchDocument, build_documents};
use earl::template::loader::load_catalog_from_dirs;

const GITHUB_HCL: &str = r#"
version = 1
provider = "github"
categories = ["scm", "issues"]

command "search_issues" {
  title = "Search Issues"
  summary = "Search issues by text"
  description = <<-EOF
Finds issues by text query.

## Example
`earl call github.search_issues --query "bug"`
EOF
  categories = ["search"]

  annotations {
    mode = "read"
    secrets = []
  }

  param "query" {
    type = "string"
    required = true
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.github.com/search/issues"
  }

  result {
    output = "Found {{ result.total_count }}"
  }
}
"#;

fn build_test_docs() -> Vec<SearchDocument> {
    let ws = common::temp_workspace();
    common::write_template(&ws.local_templates, "github.hcl", GITHUB_HCL);
    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    build_documents(&catalog)
}

#[test]
fn single_document_produced_per_command() {
    let docs = build_test_docs();
    assert_eq!(docs.len(), 1);
}

#[test]
fn document_key_is_provider_dot_command() {
    let doc = &build_test_docs()[0];
    assert_eq!(doc.key, "github.search_issues");
}

#[test]
fn document_mode_reflects_annotations_mode() {
    let doc = &build_test_docs()[0];
    assert_eq!(doc.mode, "read");
}

#[test]
fn document_categories_includes_provider_level_category() {
    let doc = &build_test_docs()[0];
    assert!(doc.categories.contains(&"scm".to_string()));
}

#[test]
fn document_categories_includes_command_level_category() {
    let doc = &build_test_docs()[0];
    assert!(doc.categories.contains(&"search".to_string()));
}

#[test]
fn document_text_includes_title() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("Search Issues"));
}

#[test]
fn document_text_includes_summary() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("Search issues by text"));
}

#[test]
fn document_text_includes_description_body() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("Finds issues by text query."));
}

#[test]
fn document_text_includes_description_example_section() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("## Example"));
}

#[test]
fn document_text_includes_operation_url() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("https://api.github.com/search/issues"));
}

#[test]
fn document_text_includes_param_spec() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("query:string"));
}

#[test]
fn document_text_includes_result_output() {
    let doc = &build_test_docs()[0];
    assert!(doc.text.contains("Found {{ result.total_count }}"));
}
