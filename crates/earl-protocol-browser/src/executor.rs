use anyhow::Result;
use chrono::Utc;
use earl_core::{ExecutionContext, ProtocolExecutor, RawExecutionResult};

use crate::PreparedBrowserCommand;
use crate::launcher::{connect_chrome, configure_page, launch_chrome};
use crate::session::{
    SessionFile, acquire_session_lock, ensure_sessions_dir, is_pid_alive, session_file_path,
    sessions_dir,
};
use crate::steps::execute_steps;

/// Browser protocol executor.
///
/// Supports two modes:
/// - **Ephemeral** (`session_id` is `None`): launches Chrome, opens a fresh
///   page, runs the steps, then closes Chrome.
/// - **Session** (`session_id` is `Some`): acquires an advisory lock on the
///   session file, reconnects to an existing Chrome instance if it is still
///   alive, otherwise launches a fresh one; runs the steps; updates the session
///   file.
pub struct BrowserExecutor;

impl ProtocolExecutor for BrowserExecutor {
    type PreparedData = PreparedBrowserCommand;

    async fn execute(
        &mut self,
        data: &PreparedBrowserCommand,
        _ctx: &ExecutionContext,
    ) -> Result<RawExecutionResult> {
        let result = run_browser_command(data).await?;
        let body = serde_json::to_vec(&result)?;
        Ok(RawExecutionResult {
            status: 0,
            url: "browser://command".into(),
            body,
            content_type: Some("application/json".into()),
        })
    }
}

/// Core execution logic — runs the browser steps and returns a JSON `Value`.
async fn run_browser_command(data: &PreparedBrowserCommand) -> Result<serde_json::Value> {
    match data.session_id.as_deref() {
        None => run_ephemeral(data).await,
        Some(session_id) => run_with_session(data, session_id).await,
    }
}

/// Launch a fresh Chrome instance, run the steps on a new page, then close.
async fn run_ephemeral(data: &PreparedBrowserCommand) -> Result<serde_json::Value> {
    let (mut browser, _ws_url) = launch_chrome(data.headless).await?;

    let page = browser.new_page("about:blank").await?;
    configure_page(&page).await?;

    let result = execute_steps(
        &page,
        &data.steps,
        data.timeout_ms,
        data.on_failure_screenshot,
    )
    .await;

    // Close Chrome regardless of step outcome.
    let _ = browser.close().await;

    result
}

/// Connect to (or launch) a Chrome instance tracked by a session file, run the
/// steps, then update the session file with the current state.
async fn run_with_session(
    data: &PreparedBrowserCommand,
    session_id: &str,
) -> Result<serde_json::Value> {
    // Ensure the sessions directory exists before acquiring the lock.
    let dir = sessions_dir()?;
    ensure_sessions_dir(&dir)?;

    // Advisory lock prevents concurrent earl invocations from clobbering the
    // same session.
    let _lock = acquire_session_lock(session_id).await?;

    let sf_path = session_file_path(session_id)?;
    let existing = SessionFile::load_from(&sf_path)?;

    // Try to reconnect to an existing Chrome instance.
    let (browser, ws_url) = if let Some(ref sf) = existing {
        if is_pid_alive(sf.pid, Some(sf.started_at)) {
            match connect_chrome(&sf.websocket_url).await {
                Ok(b) => (b, sf.websocket_url.clone()),
                Err(_) => {
                    // Stale session — launch a fresh Chrome.
                    let (b, ws) = launch_chrome(data.headless).await?;
                    (b, ws)
                }
            }
        } else {
            // PID no longer alive — launch a fresh Chrome.
            let (b, ws) = launch_chrome(data.headless).await?;
            (b, ws)
        }
    } else {
        // No session file yet — launch a fresh Chrome.
        let (b, ws) = launch_chrome(data.headless).await?;
        (b, ws)
    };

    // Reuse the first existing page or open a new one.
    let page = match browser.pages().await {
        Ok(pages) if !pages.is_empty() => pages.into_iter().next().unwrap(),
        _ => {
            let p = browser.new_page("about:blank").await?;
            configure_page(&p).await?;
            p
        }
    };

    // Persist the session file so the next invocation can reconnect.
    let target_id = page.target_id().as_ref().to_string();

    let now = Utc::now();
    let started_at = existing.as_ref().map(|sf| sf.started_at).unwrap_or(now);

    let sf = SessionFile {
        // Use 0 as a placeholder — chromiumoxide does not expose Chrome's PID
        // through its public API.
        pid: 0,
        websocket_url: ws_url,
        target_id,
        started_at,
        last_used_at: now,
        interrupted: false,
    };
    sf.save_to(&sf_path)?;

    // Run the steps.
    execute_steps(
        &page,
        &data.steps,
        data.timeout_ms,
        data.on_failure_screenshot,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_executor_implements_protocol_executor() {
        fn assert_impl<T: earl_core::ProtocolExecutor>() {}
        assert_impl::<BrowserExecutor>();
    }
}
