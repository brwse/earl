use std::path::PathBuf;

use anyhow::{Context, Result};
use chromiumoxide::browser::BrowserConfig;
use chromiumoxide::handler::Handler;
use chromiumoxide::{Browser, Page};
use futures::StreamExt;

use crate::error::BrowserError;

/// Platform-ordered list of Chrome binary candidates.
///
/// If the `EARL_BROWSER_PATH` environment variable is set, it is returned as
/// the sole candidate (no fallbacks are tried). Otherwise, a list of
/// well-known installation paths for the current platform is returned,
/// followed by any matches found on `PATH` via the `which` crate.
pub fn chrome_binary_candidates() -> Vec<PathBuf> {
    // EARL_BROWSER_PATH override takes priority and is the only result.
    if let Ok(p) = std::env::var("EARL_BROWSER_PATH") {
        return vec![PathBuf::from(p)];
    }

    let mut candidates = vec![];

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        candidates.push(PathBuf::from(
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ));
    }

    #[cfg(target_os = "linux")]
    {
        candidates.push(PathBuf::from("/usr/bin/google-chrome"));
        candidates.push(PathBuf::from("/usr/bin/google-chrome-stable"));
        candidates.push(PathBuf::from("/usr/bin/chromium-browser"));
        candidates.push(PathBuf::from("/usr/bin/chromium"));
    }

    #[cfg(target_os = "windows")]
    {
        candidates.push(PathBuf::from(
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        ));
        candidates.push(PathBuf::from(
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ));
    }

    // PATH fallbacks via `which` crate.
    for name in &["chrome", "google-chrome", "chromium", "chromium-browser"] {
        if let Ok(p) = which::which(name)
            && !candidates.contains(&p)
        {
            candidates.push(p);
        }
    }

    candidates
}

/// Find the Chrome binary, returning the first path that exists.
///
/// Returns `BrowserError::ChromeNotFound` if none of the candidates exist.
pub fn find_chrome() -> Result<PathBuf> {
    let candidates = chrome_binary_candidates();
    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }
    let paths = candidates
        .iter()
        .map(|p| format!("  - {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(BrowserError::ChromeNotFound { paths }.into())
}

/// Spawn the chromiumoxide `Handler` on a Tokio task.
///
/// The handler **must** be polled continuously — if it stops, all CDP commands
/// will deadlock. This helper spawns it as a background task that runs until
/// the handler's stream is exhausted (i.e., the browser connection closes).
fn spawn_handler(mut handler: Handler) {
    tokio::spawn(async move { while handler.next().await.is_some() {} });
}

/// Launch a new Chrome/Chromium instance and return the connected `Browser`.
///
/// The handler task is spawned automatically; callers do not need to manage it.
/// The second element of the returned tuple is the WebSocket debug URL for the
/// launched instance — useful for reconnecting or recording the session.
///
/// # Arguments
/// * `headless` — `true` runs in headless mode (default Chrome headless); `false` shows the window.
pub async fn launch_chrome(headless: bool) -> Result<(Browser, String)> {
    let chrome = find_chrome()?;

    let mut config_builder = BrowserConfig::builder()
        .chrome_executable(chrome)
        // Disable chromiumoxide's own request timeout; Earl manages timeouts externally.
        .request_timeout(std::time::Duration::from_secs(3600));

    if !headless {
        config_builder = config_builder.with_head();
    }

    let config = config_builder
        .build()
        .map_err(|e| anyhow::anyhow!("browser config error: {e}"))?;

    let (browser, handler) = Browser::launch(config).await.context("launching Chrome")?;

    spawn_handler(handler);

    let ws_url = browser.websocket_address().clone();
    Ok((browser, ws_url))
}

/// Connect to an existing Chrome instance by WebSocket (or HTTP debug) URL.
///
/// The handler task is spawned automatically.
pub async fn connect_chrome(ws_url: &str) -> Result<Browser> {
    let (browser, handler) = Browser::connect(ws_url)
        .await
        .context("connecting to Chrome CDP")?;

    spawn_handler(handler);

    Ok(browser)
}

/// Apply Earl's default page configuration after a page is created.
///
/// Currently this denies all downloads so that unexpected file saves surface
/// as an error rather than silently writing to disk.
pub async fn configure_page(page: &Page) -> Result<()> {
    use chromiumoxide::cdp::browser_protocol::browser::{
        SetDownloadBehaviorBehavior, SetDownloadBehaviorParams,
    };

    // Deny all downloads by default.
    let params = SetDownloadBehaviorParams::new(SetDownloadBehaviorBehavior::Deny);

    page.execute(params)
        .await
        .context("setting download behavior to deny")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_binary_candidates_non_empty() {
        let candidates = chrome_binary_candidates();
        assert!(!candidates.is_empty());
    }

    #[test]
    fn find_chrome_returns_result() {
        // Should either find Chrome or return a ChromeNotFound error.
        // Either outcome is valid — we just check it doesn't panic.
        let _ = find_chrome();
    }
}
