use anyhow::Result;

use crate::config::Config;
use crate::secrets::SecretManager;
use crate::template::catalog::TemplateCatalog;

use super::index::{SearchHit, build_documents};
use super::remote_openai_compat::search_remote;

pub async fn search_templates(
    query: &str,
    catalog: &TemplateCatalog,
    config: &Config,
    secrets: &SecretManager,
    limit: usize,
) -> Result<Vec<SearchHit>> {
    let documents = build_documents(catalog);

    if let Some(remote_hits) =
        search_remote(query, &documents, &config.search.remote, secrets).await?
    {
        return Ok(remote_hits.into_iter().take(limit).collect());
    }

    #[cfg(feature = "local-search")]
    {
        use super::local_fastembed::search_local;

        let query_owned = query.to_string();
        let docs_owned = documents.clone();
        let search_cfg = config.search.clone();

        let local_result = tokio::task::spawn_blocking(move || {
            search_local(&query_owned, &docs_owned, &search_cfg)
        })
        .await;

        if let Ok(Ok(hits)) = local_result {
            let mut hits = hits;
            hits.truncate(limit);
            return Ok(hits);
        }
    }

    let mut hits = lexical_fallback(query, &documents);
    hits.truncate(limit);
    Ok(hits)
}

fn lexical_fallback(query: &str, documents: &[super::index::SearchDocument]) -> Vec<SearchHit> {
    let mut hits: Vec<SearchHit> = documents
        .iter()
        .map(|doc| {
            let score = lexical_score(query, &doc.text);
            SearchHit {
                key: doc.key.clone(),
                score,
                summary: doc.summary.clone(),
            }
        })
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score));
    hits
}

fn lexical_score(query: &str, text: &str) -> f32 {
    let query_lower = query.to_ascii_lowercase();
    let text_lower = text.to_ascii_lowercase();
    let mut score = 0.0;
    for token in query_lower.split_whitespace() {
        if text_lower.contains(token) {
            score += 1.0;
        }
    }
    score
}
