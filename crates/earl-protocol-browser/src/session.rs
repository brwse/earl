use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::BrowserError;

/// Reject session IDs that could be used for path traversal or other abuse.
///
/// Allowed characters: ASCII letters, digits, hyphens, and underscores.
pub fn validate_session_id(session_id: &str) -> Result<()> {
    if session_id.is_empty() {
        return Err(anyhow::anyhow!("session_id must not be empty"));
    }
    if !session_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow::anyhow!(
            "session_id contains invalid characters; only ASCII letters, digits, hyphens, \
             and underscores are allowed"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub pid: u32,
    pub websocket_url: String,
    pub target_id: String,
    pub started_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub interrupted: bool,
}

impl SessionFile {
    pub fn load_from(path: &Path) -> Result<Option<Self>> {
        match std::fs::read_to_string(path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(f) => Ok(Some(f)),
                Err(_) => Ok(None), // corrupt — treat as stale
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("reading session file"),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let tmp = tempfile::NamedTempFile::new_in(dir).context("creating temp session file")?;
        serde_json::to_writer(&tmp, self).context("serializing session file")?;
        tmp.persist(path)
            .map_err(|e| anyhow::anyhow!("persisting session file: {}", e.error))?;
        Ok(())
    }

    pub fn delete(path: &Path) -> Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).context("deleting session file"),
        }
    }
}

pub fn ensure_sessions_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).context("creating sessions directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(dir, perms).context("setting sessions directory permissions")?;
    }
    Ok(())
}

pub fn sessions_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;
    Ok(base.config_dir().join("earl").join("browser-sessions"))
}

pub fn session_file_path(session_id: &str) -> Result<PathBuf> {
    validate_session_id(session_id)?;
    Ok(sessions_dir()?.join(format!("{session_id}.json")))
}

pub fn lock_file_path(session_id: &str) -> Result<PathBuf> {
    validate_session_id(session_id)?;
    Ok(sessions_dir()?.join(format!("{session_id}.lock")))
}

pub fn is_pid_alive(pid: u32, _started_at: Option<DateTime<Utc>>) -> bool {
    if pid == 0 {
        // pid=0 means we don't know the PID (chromiumoxide doesn't expose it);
        // return true so the caller falls through to the CDP probe.
        return true;
    }
    #[cfg(unix)]
    {
        // Reject PIDs that would wrap to negative pid_t values (e.g. u32::MAX → -1).
        // On Unix, kill(-1, 0) signals all processes and is not a PID existence check.
        let pid_t = pid as libc::pid_t;
        if pid_t <= 0 {
            return false;
        }
        // kill(pid, 0) returns 0 if process exists and we can signal it, -1 otherwise.
        let result = unsafe { libc::kill(pid_t, 0) };
        result == 0
    }
    #[cfg(not(unix))]
    {
        // On non-unix, skip PID check; rely solely on CDP probe.
        let _ = pid;
        true
    }
}

pub async fn acquire_session_lock(session_id: &str) -> Result<tokio::fs::File> {
    use fs4::tokio::AsyncFileExt;

    validate_session_id(session_id)?;
    let dir = sessions_dir()?;
    ensure_sessions_dir(&dir)?;
    let lock_path = dir.join(format!("{session_id}.lock"));

    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&lock_path)
        .await
        .context("opening session lock file")?;

    match file.try_lock_exclusive() {
        Ok(()) => Ok(file),
        Err(_) => {
            // Try to read the PID from the lock file content (best-effort).
            let pid = tokio::fs::read_to_string(&lock_path)
                .await
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .unwrap_or(0);
            Err(BrowserError::SessionLocked {
                session_id: session_id.to_string(),
                pid,
            }
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trip_session_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.json");
        let orig = SessionFile {
            pid: 12345,
            websocket_url: "ws://127.0.0.1:9222/devtools/browser/xyz".into(),
            target_id: "T42".into(),
            started_at: Utc::now(),
            last_used_at: Utc::now(),
            interrupted: false,
        };
        orig.save_to(&path).unwrap();
        let loaded = SessionFile::load_from(&path).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.pid, 12345);
        assert_eq!(loaded.target_id, "T42");
        assert_eq!(
            loaded.websocket_url,
            "ws://127.0.0.1:9222/devtools/browser/xyz"
        );
        assert!(!loaded.interrupted);
    }

    #[test]
    fn corrupt_json_returns_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, b"not json {{{{").unwrap();
        assert!(SessionFile::load_from(&path).unwrap().is_none());
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        assert!(SessionFile::load_from(&path).unwrap().is_none());
    }

    #[test]
    fn delete_removes_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(&path, b"{}").unwrap();
        SessionFile::delete(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn delete_nonexistent_is_ok() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        assert!(SessionFile::delete(&path).is_ok());
    }

    #[test]
    fn session_dir_creates_with_correct_permissions() {
        let dir = TempDir::new().unwrap();
        let sessions_dir = dir.path().join("sessions");
        ensure_sessions_dir(&sessions_dir).unwrap();
        assert!(sessions_dir.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&sessions_dir).unwrap();
            assert_eq!(meta.permissions().mode() & 0o777, 0o700);
        }
    }

    #[test]
    fn is_pid_alive_returns_false_for_impossible_pid() {
        // PID u32::MAX is virtually guaranteed not to exist
        let alive = is_pid_alive(u32::MAX, None);
        assert!(!alive);
    }

    #[test]
    fn validate_session_id_accepts_safe_ids() {
        assert!(validate_session_id("my-session").is_ok());
        assert!(validate_session_id("session_123").is_ok());
        assert!(validate_session_id("ABC-def-0").is_ok());
    }

    #[test]
    fn validate_session_id_rejects_path_traversal() {
        assert!(validate_session_id("../../etc/passwd").is_err());
        assert!(validate_session_id("../sibling").is_err());
        assert!(validate_session_id("foo/bar").is_err());
        assert!(validate_session_id("foo\\bar").is_err());
    }

    #[test]
    fn validate_session_id_rejects_empty() {
        assert!(validate_session_id("").is_err());
    }
}
