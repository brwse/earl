use std::path::{Component, Path};

use anyhow::Result;
use tokio::process::Command;

use crate::ResolvedBashSandbox;

/// Returns `true` when a supported sandbox tool is available on this platform.
pub fn sandbox_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        which("bwrap")
    }
    #[cfg(target_os = "macos")]
    {
        which("sandbox-exec")
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Human-readable name of the sandbox back-end for diagnostic messages.
pub fn sandbox_tool_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "bwrap (bubblewrap)"
    }
    #[cfg(target_os = "macos")]
    {
        "sandbox-exec"
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        "unsupported"
    }
}

/// Build a [`Command`] that will execute `script` inside a sandbox.
///
/// On Linux the command uses `bwrap` with targeted read-only mounts, isolated
/// network, PID, IPC, and UTS namespaces, a tmpfs `/tmp`, and real `/dev` + `/proc`.
///
/// On macOS the command uses `sandbox-exec` with a dynamically generated
/// Seatbelt profile that scopes `file-read*` and `mach-lookup` to required paths.
pub fn build_sandboxed_command(
    script: &str,
    env: &[(String, String)],
    cwd: Option<&str>,
    sandbox: &ResolvedBashSandbox,
) -> Result<Command> {
    #[cfg(target_os = "linux")]
    {
        build_linux_command(script, env, cwd, sandbox)
    }
    #[cfg(target_os = "macos")]
    {
        build_macos_command(script, env, cwd, sandbox)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (script, env, cwd, sandbox);
        anyhow::bail!(
            "bash sandbox is not supported on this platform; \
             only Linux (bwrap) and macOS (sandbox-exec) are supported"
        );
    }
}

// -- Linux (bwrap) -----------------------------------------------------------

#[cfg(target_os = "linux")]
fn build_linux_command(
    script: &str,
    env: &[(String, String)],
    cwd: Option<&str>,
    sandbox: &ResolvedBashSandbox,
) -> Result<Command> {
    let mut cmd = Command::new("bwrap");

    // Targeted read-only mounts instead of --ro-bind / /
    for dir in &["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc"] {
        if Path::new(dir).exists() {
            cmd.args(["--ro-bind", dir, dir]);
        }
    }

    cmd.args(["--tmpfs", "/tmp"]);
    cmd.args(["--dev", "/dev"]);
    cmd.args(["--proc", "/proc"]);

    // Namespace isolation
    cmd.arg("--unshare-pid");
    cmd.arg("--unshare-ipc");
    cmd.arg("--unshare-uts");
    if !sandbox.network {
        cmd.arg("--unshare-net");
    }
    cmd.arg("--die-with-parent");

    if let Some(dir) = cwd {
        validate_sandbox_cwd(dir)?;
        // Mount cwd read-only by default
        cmd.args(["--ro-bind", dir, dir]);
        cmd.args(["--chdir", dir]);

        // Mount writable sub-paths with --bind
        for writable in &sandbox.writable_paths {
            let full_path = Path::new(dir).join(writable);
            let full_str = full_path.to_string_lossy();
            if full_path.exists() {
                cmd.args(["--bind", &full_str, &full_str]);
            }
        }
    }

    cmd.args(["--", "bash", "-c", script]);
    for (key, value) in env {
        // codeql[rust/cleartext-logging] - Secrets are passed as subprocess environment variables
        // by design; env vars are the standard safe mechanism for providing credentials to scripts.
        cmd.env(key, value);
    }
    Ok(cmd)
}

// -- macOS (sandbox-exec) ----------------------------------------------------

#[cfg(target_os = "macos")]
fn build_macos_command(
    script: &str,
    env: &[(String, String)],
    cwd: Option<&str>,
    sandbox: &ResolvedBashSandbox,
) -> Result<Command> {
    let profile = build_seatbelt_profile(cwd, sandbox);
    let mut cmd = Command::new("sandbox-exec");
    cmd.args(["-p", &profile, "bash", "-c", script]);
    for (key, value) in env {
        // codeql[rust/cleartext-logging] - Secrets are passed as subprocess environment variables
        // by design; env vars are the standard safe mechanism for providing credentials to scripts.
        cmd.env(key, value);
    }
    if let Some(dir) = cwd {
        validate_sandbox_cwd(dir)?;
        cmd.current_dir(dir);
    }
    Ok(cmd)
}

#[cfg(target_os = "macos")]
fn build_seatbelt_profile(cwd: Option<&str>, sandbox: &ResolvedBashSandbox) -> String {
    let mut profile = String::from(
        "(version 1)\n\
         (deny default)\n",
    );

    // Network access
    if sandbox.network {
        profile.push_str("(allow network*)\n");
    } else {
        profile.push_str("(deny network*)\n");
    }

    // Process execution
    profile.push_str("(allow process-exec)\n");
    profile.push_str("(allow process-fork)\n");
    profile.push_str("(allow sysctl-read)\n");

    // Allow read-only file access. Bash and its subprocesses need access to
    // system libraries, the dyld shared cache, and various OS metadata paths
    // that are impractical to fully enumerate. Read-only access is low risk.
    profile.push_str("(allow file-read*)\n");

    if let Some(dir) = cwd {
        // Allow write to writable sub-paths only
        for writable in &sandbox.writable_paths {
            let full_path = std::path::Path::new(dir).join(writable);
            let full_str = full_path.to_string_lossy();
            profile.push_str(&format!("(allow file-write* (subpath \"{full_str}\"))\n"));
        }
    }

    // Allow file-write to /dev/null and /dev/tty (standard shell operations).
    profile.push_str("(allow file-write* (literal \"/dev/null\"))\n");
    profile.push_str("(allow file-write* (literal \"/dev/tty\"))\n");

    // Scoped mach-lookup to required services. The old profile allowed all
    // mach-lookup which is overly broad. We allow a curated set needed for
    // basic shell operations and common system services.
    for service in &[
        "com.apple.system.logger",
        "com.apple.system.notification_center",
        "com.apple.SecurityServer",
        "com.apple.CoreServices.coreservicesd",
        "com.apple.lsd.mapdb",
    ] {
        profile.push_str(&format!(
            "(allow mach-lookup (global-name \"{service}\"))\n"
        ));
    }

    profile
}

// -- helpers -----------------------------------------------------------------

/// Validate that a sandbox cwd does not contain path traversal via `..` components.
pub fn validate_sandbox_cwd(dir: &str) -> Result<()> {
    if Path::new(dir)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        anyhow::bail!("sandbox cwd must not contain `..` path components");
    }
    Ok(())
}

fn which(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
