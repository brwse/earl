use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config;

use super::catalog::{TemplateCatalog, TemplateCatalogEntry, TemplateScope, TemplateSource};
use super::files::template_files_in_dir;
use super::parser::parse_template_hcl;
use super::schema::TemplateFile;
use super::validator::validate_template_file;

// Re-export for callers that import is_template_file from this module (e.g. doctor.rs).
pub use super::files::is_template_file;

pub fn load_catalog(cwd: &Path) -> Result<TemplateCatalog> {
    let global_dir = config::global_templates_dir();
    let local_dir = config::local_templates_dir(cwd);
    load_catalog_with_cache(&global_dir, &local_dir, &config::catalog_cache_path())
}

fn load_catalog_with_cache(
    global_dir: &Path,
    local_dir: &Path,
    cache_path: &Path,
) -> Result<TemplateCatalog> {
    let fingerprint = super::cache::collect_fingerprint(global_dir, local_dir)?;

    if let Some(catalog) = super::cache::try_load_cache(cache_path, &fingerprint) {
        return Ok(catalog);
    }

    let catalog = load_catalog_from_dirs(global_dir, local_dir)?;
    // Best-effort: ignore write errors (cache is an optimization, not a requirement)
    let _ = super::cache::save_cache(cache_path, &fingerprint, &catalog);
    Ok(catalog)
}

pub fn load_catalog_from_dirs(global_dir: &Path, local_dir: &Path) -> Result<TemplateCatalog> {
    let mut catalog = TemplateCatalog::empty();

    for file in template_files_in_dir(global_dir)? {
        load_file_into_catalog(&file, TemplateScope::Global, &mut catalog)?;
    }

    for file in template_files_in_dir(local_dir)? {
        load_file_into_catalog(&file, TemplateScope::Local, &mut catalog)?;
    }

    Ok(catalog)
}

pub fn validate_all(cwd: &Path) -> Result<Vec<PathBuf>> {
    let global_dir = config::global_templates_dir();
    let local_dir = config::local_templates_dir(cwd);
    validate_all_from_dirs(&global_dir, &local_dir)
}

pub fn validate_all_from_dirs(global_dir: &Path, local_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    for f in template_files_in_dir(global_dir)? {
        files.insert(f);
    }
    for f in template_files_in_dir(local_dir)? {
        files.insert(f);
    }

    for file in &files {
        let content = fs::read_to_string(file)
            .with_context(|| format!("failed reading template {}", file.display()))?;
        let base_dir = file.parent().unwrap_or(Path::new("."));
        let parsed: TemplateFile = parse_template_hcl(&content, base_dir)
            .with_context(|| format!("invalid HCL in {}", file.display()))?;
        validate_template_file(&parsed)
            .with_context(|| format!("validation failed for {}", file.display()))?;
    }

    Ok(files.into_iter().collect())
}

fn load_file_into_catalog(
    path: &Path,
    scope: TemplateScope,
    catalog: &mut TemplateCatalog,
) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed reading template {}", path.display()))?;
    let base_dir = path.parent().unwrap_or(Path::new("."));
    let parsed: TemplateFile = parse_template_hcl(&content, base_dir)
        .with_context(|| format!("invalid HCL in {}", path.display()))?;

    validate_template_file(&parsed)
        .with_context(|| format!("validation failed for {}", path.display()))?;

    for (command_name, template) in parsed.commands {
        let key = format!("{}.{}", parsed.provider, command_name);

        let mut categories = parsed.categories.clone();
        for category in &template.categories {
            if !categories.iter().any(|c| c == category) {
                categories.push(category.clone());
            }
        }

        let entry = TemplateCatalogEntry {
            key: key.clone(),
            provider: parsed.provider.clone(),
            command: command_name,
            title: template.title.clone(),
            summary: template.summary.clone(),
            description: template.description.clone(),
            categories,
            mode: template.annotations.mode,
            source: TemplateSource {
                path: path.to_path_buf(),
                scope,
            },
            template,
        };

        catalog.upsert(key, entry);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_bash_template(dir: &TempDir, provider: &str, command: &str) {
        let tdir = dir.path().join("templates");
        std::fs::create_dir_all(&tdir).unwrap();
        std::fs::write(
            tdir.join(format!("{provider}.hcl")),
            format!(
                r#"version = 1
provider = "{provider}"
command "{command}" {{
  title = "T"
  summary = "S"
  description = "D"
  operation {{
    protocol = "bash"
    bash {{
      script = "echo hi"
    }}
  }}
}}
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn load_catalog_returns_correct_entries() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        write_bash_template(&tmp, "myprovider", "mycommand");

        let catalog = load_catalog_from_dirs(global.path(), &tmp.path().join("templates")).unwrap();
        assert!(catalog.get("myprovider.mycommand").is_some());
    }

    #[test]
    fn load_catalog_is_idempotent_across_two_calls() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        write_bash_template(&tmp, "myprovider2", "cmd");

        let local = tmp.path().join("templates");
        let c1 = load_catalog_from_dirs(global.path(), &local).unwrap();
        let c2 = load_catalog_from_dirs(global.path(), &local).unwrap();

        let e1 = c1.get("myprovider2.cmd").unwrap();
        let e2 = c2.get("myprovider2.cmd").unwrap();
        assert_eq!(e1.title, e2.title);
    }

    #[test]
    fn load_catalog_writes_cache_on_miss_and_hits_on_second_call() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let cache_path = cache_dir.path().join("catalog-test.bin");
        write_bash_template(&tmp, "myprovider3", "cached_cmd");

        let local = tmp.path().join("templates");

        // First call: cache miss — parses HCL and writes cache.
        assert!(!cache_path.exists());
        let c1 = load_catalog_with_cache(global.path(), &local, &cache_path).unwrap();
        assert!(c1.get("myprovider3.cached_cmd").is_some());
        assert!(
            cache_path.exists(),
            "cache file should have been written after miss"
        );

        // Second call with unchanged files: cache hit — returns same catalog.
        let c2 = load_catalog_with_cache(global.path(), &local, &cache_path).unwrap();
        assert!(c2.get("myprovider3.cached_cmd").is_some());
        assert_eq!(
            c1.get("myprovider3.cached_cmd").unwrap().title,
            c2.get("myprovider3.cached_cmd").unwrap().title
        );
    }
}
