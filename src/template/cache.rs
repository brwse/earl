use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::template::catalog::TemplateCatalog;

pub const CACHE_VERSION: u32 = 1;

/// Serialized catalog cache file stored at ~/.cache/earl/catalog-1.bin.
#[derive(Serialize, Deserialize)]
pub struct CacheFile {
    pub version: u32,
    /// Sorted list of (absolute_path, mtime_unix_secs) for every .hcl file.
    pub fingerprint: Vec<(PathBuf, u64)>,
    pub catalog: TemplateCatalog,
}

/// Collects (absolute_path, mtime_unix_secs) for every .hcl file in both
/// directories, sorted by path. This is a cheap readdir-only operation —
/// file contents are not read.
pub fn collect_fingerprint(global_dir: &Path, local_dir: &Path) -> Result<Vec<(PathBuf, u64)>> {
    let mut entries: Vec<(PathBuf, u64)> = Vec::new();
    for dir in [global_dir, local_dir] {
        for path in super::loader::template_files_in_dir(dir)? {
            let mtime = std::fs::metadata(&path)?
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            entries.push((path, mtime));
        }
    }
    // Sort for stable comparison; dedup keeps first occurrence (global before local).
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries.dedup_by(|a, b| a.0 == b.0);
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cache_file_roundtrips_bincode() {
        let original = CacheFile {
            version: CACHE_VERSION,
            fingerprint: vec![(PathBuf::from("/tmp/foo.hcl"), 1_700_000_000u64)],
            catalog: crate::template::catalog::TemplateCatalog::empty(),
        };
        let bytes = bincode::serialize(&original).expect("serialize");
        let decoded: CacheFile = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(decoded.version, CACHE_VERSION);
        assert_eq!(decoded.fingerprint, original.fingerprint);
        assert_eq!(decoded.catalog.entries.len(), 0);
    }

    #[test]
    fn empty_dirs_give_empty_fingerprint() {
        let tmp = tempfile::tempdir().unwrap();
        let fp = collect_fingerprint(tmp.path(), tmp.path()).unwrap();
        assert!(fp.is_empty());
    }

    #[test]
    fn fingerprint_changes_when_file_added() {
        let tmp = tempfile::tempdir().unwrap();
        let fp1 = collect_fingerprint(tmp.path(), tmp.path()).unwrap();

        std::fs::write(tmp.path().join("new.hcl"), "content").unwrap();
        let fp2 = collect_fingerprint(tmp.path(), tmp.path()).unwrap();

        assert_ne!(fp1, fp2);
        assert_eq!(fp2.len(), 1);
    }
}
