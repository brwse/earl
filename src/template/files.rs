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
    // Canonicalize immediately so all subsequent path operations use a clean,
    // absolute path. Treat a non-existent directory as empty (no templates).
    let dir = match dir.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(anyhow::Error::from(e).context(format!(
                "failed to canonicalize template directory {}",
                dir.display()
            )));
        }
    };
    let mut files = Vec::new();
    collect_template_files(&dir, &dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_template_files(dir: &Path, root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("failed listing template directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed inspecting template path {}", path.display()))?;
        if file_type.is_dir() {
            // Canonicalize before recursing; skip entries that escape the root
            // (e.g. symlinks pointing outside the template directory).
            if let Ok(canonical) = path.canonicalize() {
                if canonical.starts_with(root) {
                    collect_template_files(&canonical, root, files)?;
                }
            }
            continue;
        }
        if file_type.is_file() && is_template_file(&path) {
            // Canonicalize before storing; skip entries that escape the root.
            if let Ok(canonical) = path.canonicalize() {
                if canonical.starts_with(root) {
                    files.push(canonical);
                }
            }
        }
    }
    Ok(())
}
