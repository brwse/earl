#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error(
        "browser step {step} ({action}) failed: element not found — {selector} — completed {completed} of {total} steps"
    )]
    ElementNotFound {
        step: usize,
        action: String,
        selector: String,
        completed: usize,
        total: usize,
    },

    #[error(
        "browser step {step} ({action}) failed: element not interactable — {selector} — completed {completed} of {total} steps"
    )]
    ElementNotInteractable {
        step: usize,
        action: String,
        selector: String,
        completed: usize,
        total: usize,
    },

    #[error("browser step {step} (navigate) failed: {reason}")]
    NavigationFailed { step: usize, reason: String },

    #[error("browser step {step} ({action}) assertion failed: {message}")]
    AssertionFailed {
        step: usize,
        action: String,
        message: String,
    },

    #[error("browser renderer crashed at step {step}")]
    RendererCrashed { step: usize },

    #[error(
        "browser step {step}: a dialog is blocking the page — add a handle_dialog step before this one"
    )]
    DialogBlocking { step: usize },

    #[error(
        "browser step {step}: a download was triggered — use a download step with save_to to handle it explicitly"
    )]
    DownloadBlocked { step: usize },

    #[error(
        "browser step {step} (click): a new tab was opened — add a tabs step with operation=\"select\" to switch to it"
    )]
    NewTabOpened { step: usize },

    #[error("browser step {step} ({action}) timed out after {timeout_ms}ms")]
    Timeout {
        step: usize,
        action: String,
        timeout_ms: u64,
    },

    #[error("ref \"{ref_id}\" no longer exists in the accessibility tree (used in {action})")]
    StaleRef { ref_id: String, action: String },

    #[error(
        "URL scheme \"{scheme}\" is not allowed in navigate — only http and https are permitted"
    )]
    DisallowedScheme { scheme: String },

    // session_id intentionally omitted from the Display string to avoid CWE-532 (cleartext
    // logging of sensitive identifiers). The caller already knows which session they requested.
    #[error("browser session is locked by another earl process (PID {pid})")]
    SessionLocked { session_id: String, pid: u32 },

    #[error(
        "Chrome not found — install Chrome/Chromium or set EARL_BROWSER_PATH\nPaths tried:\n{paths}"
    )]
    ChromeNotFound { paths: String },

    #[error("browser session lost during step {step} ({action}): {reason}")]
    SessionLost {
        step: usize,
        action: String,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_include_context() {
        let e = BrowserError::ElementNotFound {
            step: 2,
            action: "click".into(),
            selector: "#submit".into(),
            completed: 1,
            total: 5,
        };
        let msg = e.to_string();
        assert!(msg.contains("step 2"));
        assert!(msg.contains("click"));
        assert!(msg.contains("#submit"));
        assert!(msg.contains("completed 1 of 5"));

        let e2 = BrowserError::DisallowedScheme {
            scheme: "file".into(),
        };
        assert!(e2.to_string().contains("file"));
        assert!(e2.to_string().contains("http"));
    }
}
