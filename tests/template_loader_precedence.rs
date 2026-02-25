mod common;

use earl::template::catalog::TemplateScope;
use earl::template::loader::load_catalog_from_dirs;

const GLOBAL_OVERRIDE_HCL: &str = r#"
version = 1
provider = "github"
categories = ["global_cat"]

command "search_issues" {
  title = "Global Search"
  summary = "Global search command"
  description = "global version"
  categories = ["global_cmd"]

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.github.com/search/issues"
  }

  result {
    output = "global"
  }
}
"#;

const MULTI_COMMAND_HCL: &str = r#"
version = 1
provider = "github"
categories = ["scm"]

command "search_issues" {
  title = "Search Issues"
  summary = "Search issues command"
  description = "Search issues in repositories"

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.github.com/search/issues"
  }

  result {
    output = "ok"
  }
}

command "create_issue" {
  title = "Create Issue"
  summary = "Create issue command"
  description = "Create an issue in a repository"

  annotations {
    mode = "write"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "POST"
    url = "https://api.github.com/repos/org/repo/issues"
  }

  result {
    output = "ok"
  }
}
"#;

const LOCAL_OVERRIDE_HCL: &str = r#"
version = 1
provider = "github"
categories = ["local_cat"]

command "search_issues" {
  title = "Local Search"
  summary = "Local search command"
  description = "local version"
  categories = ["local_cmd"]

  annotations {
    mode = "read"
    secrets = []
  }

  operation {
    protocol = "http"
    method = "GET"
    url = "https://api.github.com/search/issues"
  }

  result {
    output = "local"
  }
}
"#;

#[test]
fn local_title_overrides_global_for_same_command_key() {
    let ws = common::temp_workspace();
    common::write_template(&ws.global_templates, "github.hcl", GLOBAL_OVERRIDE_HCL);
    common::write_template(&ws.local_templates, "github.hcl", LOCAL_OVERRIDE_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    let entry = catalog.get("github.search_issues").unwrap();

    assert_eq!(entry.title, "Local Search");
}

#[test]
fn local_summary_overrides_global_for_same_command_key() {
    let ws = common::temp_workspace();
    common::write_template(&ws.global_templates, "github.hcl", GLOBAL_OVERRIDE_HCL);
    common::write_template(&ws.local_templates, "github.hcl", LOCAL_OVERRIDE_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    let entry = catalog.get("github.search_issues").unwrap();

    assert_eq!(entry.summary, "Local search command");
}

#[test]
fn local_description_overrides_global_for_same_command_key() {
    let ws = common::temp_workspace();
    common::write_template(&ws.global_templates, "github.hcl", GLOBAL_OVERRIDE_HCL);
    common::write_template(&ws.local_templates, "github.hcl", LOCAL_OVERRIDE_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    let entry = catalog.get("github.search_issues").unwrap();

    assert_eq!(entry.description, "local version");
}

#[test]
fn local_scope_reported_when_local_overrides_global() {
    let ws = common::temp_workspace();
    common::write_template(&ws.global_templates, "github.hcl", GLOBAL_OVERRIDE_HCL);
    common::write_template(&ws.local_templates, "github.hcl", LOCAL_OVERRIDE_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    let entry = catalog.get("github.search_issues").unwrap();

    assert_eq!(entry.source.scope, TemplateScope::Local);
}

#[test]
fn local_provider_categories_override_global_for_same_command_key() {
    let ws = common::temp_workspace();
    common::write_template(&ws.global_templates, "github.hcl", GLOBAL_OVERRIDE_HCL);
    common::write_template(&ws.local_templates, "github.hcl", LOCAL_OVERRIDE_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    let entry = catalog.get("github.search_issues").unwrap();

    assert!(entry.categories.contains(&"local_cat".to_string()));
}

#[test]
fn local_command_categories_override_global_for_same_command_key() {
    let ws = common::temp_workspace();
    common::write_template(&ws.global_templates, "github.hcl", GLOBAL_OVERRIDE_HCL);
    common::write_template(&ws.local_templates, "github.hcl", LOCAL_OVERRIDE_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    let entry = catalog.get("github.search_issues").unwrap();

    assert!(entry.categories.contains(&"local_cmd".to_string()));
}

#[test]
fn first_command_loaded_from_multi_command_file() {
    let ws = common::temp_workspace();
    common::write_template(&ws.local_templates, "github.hcl", MULTI_COMMAND_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    assert!(catalog.get("github.search_issues").is_some());
}

#[test]
fn second_command_loaded_from_multi_command_file() {
    let ws = common::temp_workspace();
    common::write_template(&ws.local_templates, "github.hcl", MULTI_COMMAND_HCL);

    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();
    assert!(catalog.get("github.create_issue").is_some());
}
