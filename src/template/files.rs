use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn is_template_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s == "hcl")
        .unwrap_or(false)
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
