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

    let provider_environments = parsed.environments.clone();

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
            provider_environments: provider_environments.clone(),
        };

        catalog.upsert(key, entry);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_env_template(dir: &TempDir) {
        let tdir = dir.path().join("templates");
        std::fs::create_dir_all(&tdir).unwrap();
        std::fs::write(
            tdir.join("envtest.hcl"),
            r#"version = 1
provider = "envtest"

environments {
  default = "production"
  secrets = []
  production { base_url = "https://prod.example.com" }
  staging    { base_url = "https://staging.example.com" }
}

command "ping" {
  title = "Ping"
  summary = "Ping"
  description = "Ping"
  annotations {
    mode = "read"
    secrets = []
  }
  operation {
    protocol = "bash"
    bash { script = "echo {{ vars.base_url }}" }
  }
}
"#,
        )
        .unwrap();
    }

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
    fn command_accessible_by_provider_dot_command_key() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        write_bash_template(&tmp, "myprovider", "mycommand");

        let catalog = load_catalog_from_dirs(global.path(), &tmp.path().join("templates")).unwrap();
        assert!(catalog.get("myprovider.mycommand").is_some());
    }

    #[test]
    fn provider_environments_default_field_stored_in_catalog_entry() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        write_env_template(&tmp);

        let tdir = tmp.path().join("templates");
        let catalog = load_catalog_from_dirs(global.path(), &tdir).unwrap();
        let entry = catalog.get("envtest.ping").expect("entry should exist");
        let envs = entry
            .provider_environments
            .as_ref()
            .expect("provider_environments should be set");
        assert_eq!(envs.default.as_deref(), Some("production"));
    }

    #[test]
    fn provider_environments_production_key_stored_in_catalog_entry() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        write_env_template(&tmp);

        let tdir = tmp.path().join("templates");
        let catalog = load_catalog_from_dirs(global.path(), &tdir).unwrap();
        let entry = catalog.get("envtest.ping").expect("entry should exist");
        let envs = entry
            .provider_environments
            .as_ref()
            .expect("provider_environments should be set");
        assert!(envs.environments.contains_key("production"));
    }

    #[test]
    fn provider_environments_staging_key_stored_in_catalog_entry() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        write_env_template(&tmp);

        let tdir = tmp.path().join("templates");
        let catalog = load_catalog_from_dirs(global.path(), &tdir).unwrap();
        let entry = catalog.get("envtest.ping").expect("entry should exist");
        let envs = entry
            .provider_environments
            .as_ref()
            .expect("provider_environments should be set");
        assert!(envs.environments.contains_key("staging"));
    }

    #[test]
    fn cache_file_written_after_catalog_load() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let cache_path = cache_dir.path().join("catalog-test.bin");
        write_bash_template(&tmp, "myprovider3", "cached_cmd");

        let local = tmp.path().join("templates");
        load_catalog_with_cache(global.path(), &local, &cache_path).unwrap();
        assert!(
            cache_path.exists(),
            "cache file should have been written after miss"
        );
    }

    #[test]
    fn second_load_returns_same_catalog_entry() {
        let tmp = TempDir::new().unwrap();
        let global = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let cache_path = cache_dir.path().join("catalog-test.bin");
        write_bash_template(&tmp, "myprovider3", "cached_cmd");

        let local = tmp.path().join("templates");
        let c1 = load_catalog_with_cache(global.path(), &local, &cache_path).unwrap();
        let c2 = load_catalog_with_cache(global.path(), &local, &cache_path).unwrap();
        assert_eq!(
            c1.get("myprovider3.cached_cmd").unwrap().title,
            c2.get("myprovider3.cached_cmd").unwrap().title
        );
    }
}
