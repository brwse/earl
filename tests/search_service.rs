mod common;

use earl::config::Config;
use earl::search::service::search_templates;
use earl::template::loader::load_catalog_from_dirs;
use httpmock::prelude::*;
use secrecy::SecretString;

fn sample_templates() -> &'static str {
    r#"
version = 1
provider = "github"
categories = ["scm"]

command "search_issues" {
  title = "Search Issues"
  summary = "Search GitHub issues"
  description = "Finds GitHub issues that match a query."
  categories = ["search", "issues"]

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
  summary = "Create issue in a repository"
  description = "Creates a GitHub issue in a target repository."
  categories = ["write", "issues"]

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
"#
}

#[tokio::test]
async fn prefers_remote_results_when_remote_search_succeeds() {
    let ws = common::temp_workspace();
    common::write_template(&ws.local_templates, "github.hcl", sample_templates());
    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();

    let server = MockServer::start_async().await;
    let embeddings_mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/embeddings");
            then.status(200).json_body_obj(&serde_json::json!({
                "data": [
                    {"embedding": [1.0, 0.0]},
                    {"embedding": [1.0, 0.0]},
                    {"embedding": [0.0, 1.0]}
                ]
            }));
        })
        .await;

    let rerank_mock = server
        .mock_async(|when, then| {
            when.method(POST).path("/rerank");
            then.status(200).json_body_obj(&serde_json::json!({
                "data": [
                    {"index": 0, "score": 0.99},
                    {"index": 1, "score": 0.75}
                ]
            }));
        })
        .await;

    let mut config = Config::default();
    config.search.remote.enabled = true;
    config.search.remote.base_url = Some(server.base_url());
    config.search.remote.api_key_secret = Some("search.api_key".to_string());
    config.search.remote.embeddings_path = "/embeddings".to_string();
    config.search.remote.rerank_path = "/rerank".to_string();

    let secrets_index_path = ws.root.path().join("state/secrets-index.json");
    let secrets = common::in_memory_secret_manager(&secrets_index_path);
    secrets
        .set(
            "search.api_key",
            SecretString::new("k-test".to_string().into()),
        )
        .unwrap();

    let hits = search_templates("create issue", &catalog, &config, &secrets, 5)
        .await
        .unwrap();

    embeddings_mock.assert_async().await;
    rerank_mock.assert_async().await;
    assert_eq!(hits[0].key, "github.create_issue");
}

#[tokio::test]
async fn falls_back_when_remote_fails_and_local_models_are_invalid() {
    let ws = common::temp_workspace();
    common::write_template(&ws.local_templates, "github.hcl", sample_templates());
    let catalog = load_catalog_from_dirs(&ws.global_templates, &ws.local_templates).unwrap();

    let server = MockServer::start_async().await;

    let mut config = Config::default();
    config.search.remote.enabled = true;
    config.search.remote.base_url = Some(server.base_url());
    config.search.remote.api_key_secret = Some("search.api_key".to_string());
    config.search.remote.embeddings_path = "/embeddings".to_string();
    config.search.remote.rerank_path = "/rerank".to_string();
    config.search.local.embedding_model = "invalid-model".to_string();
    config.search.local.reranker_model = "invalid-model".to_string();

    let secrets_index_path = ws.root.path().join("state/secrets-index.json");
    let secrets = common::in_memory_secret_manager(&secrets_index_path);
    secrets
        .set(
            "search.api_key",
            SecretString::new("k-test".to_string().into()),
        )
        .unwrap();

    let hits = search_templates("create issue", &catalog, &config, &secrets, 2)
        .await
        .unwrap();

    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].key, "github.create_issue");
}
