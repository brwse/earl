use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config;

use super::catalog::{TemplateCatalog, TemplateCatalogEntry, TemplateScope, TemplateSource};
use super::parser::parse_template_hcl;
use super::schema::TemplateFile;
use super::validator::validate_template_file;

pub fn load_catalog(cwd: &Path) -> Result<TemplateCatalog> {
    let global_dir = config::global_templates_dir();
    let local_dir = config::local_templates_dir(cwd);
    load_catalog_from_dirs(&global_dir, &local_dir)
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

pub(super) fn template_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let dir = dir.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize template directory {}",
            dir.display()
        )
    })?;
    let mut files = Vec::new();
    collect_template_files(&dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_template_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed listing template directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed inspecting template path {}", path.display()))?;
        if file_type.is_dir() {
            collect_template_files(&path, files)?;
            continue;
        }
        if file_type.is_file() && is_template_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

pub fn is_template_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s == "hcl")
        .unwrap_or(false)
}
