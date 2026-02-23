use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};

use earl_core::{ExecutionContext, RawExecutionResult, StreamChunk, StreamMeta};

use crate::PreparedBashScript;
use crate::sandbox::{build_sandboxed_command, sandbox_available, sandbox_tool_name};

/// Execute a single bash script inside a sandbox and return the result.
pub async fn execute_bash_once(
    data: &PreparedBashScript,
    ctx: &ExecutionContext,
) -> Result<RawExecutionResult> {
    if !sandbox_available() {
        bail!(
            "bash sandbox tool ({}) is not available on this system; \
             install it or disable the bash feature",
            sandbox_tool_name()
        );
    }

    let mut command =
        build_sandboxed_command(&data.script, &data.env, data.cwd.as_deref(), &data.sandbox)?;

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    if data.stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    let max_memory = data.sandbox.max_memory_bytes;
    let max_cpu_secs = data.sandbox.max_cpu_time_ms.map(|ms| (ms + 999) / 1000);

    // SAFETY: setsid() creates a new session / process group so that we can
    // kill the entire group on timeout without leaking children.
    // setrlimit() calls are safe in a post-fork, pre-exec context.
    unsafe {
        command.pre_exec(move || {
            libc::setsid();
            if let Some(bytes) = max_memory {
                let rlim = libc::rlimit {
                    rlim_cur: bytes as libc::rlim_t,
                    rlim_max: bytes as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            if let Some(secs) = max_cpu_secs {
                // Note: RLIMIT_CPU grace period sends SIGXCPU then SIGKILL ~1s later.
                let rlim = libc::rlimit {
                    rlim_cur: secs as libc::rlim_t,
                    rlim_max: secs as libc::rlim_t,
                };
                if libc::setrlimit(libc::RLIMIT_CPU, &rlim) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let mut child = command
        .spawn()
        .context("failed spawning sandboxed bash command")?;

    let pid = child
        .id()
        .ok_or_else(|| anyhow::anyhow!("failed to obtain PID of spawned bash process"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed capturing bash stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed capturing bash stderr"))?;

    // Use sandbox output limit if set, otherwise fall back to transport limit.
    let max_bytes = data
        .sandbox
        .max_output_bytes
        .unwrap_or(ctx.transport.max_response_bytes);

    let stdout_reader =
        tokio::spawn(async move { read_stream_limited(stdout, max_bytes, "stdout").await });
    let stderr_reader =
        tokio::spawn(async move { read_stream_limited(stderr, max_bytes, "stderr").await });

    // Write stdin if present, then drop the handle so the child sees EOF.
    if let Some(input) = &data.stdin
        && let Some(mut stdin_handle) = child.stdin.take()
    {
        stdin_handle
            .write_all(input.as_bytes())
            .await
            .context("failed writing stdin to bash process")?;
    }

    // Use sandbox timeout if set, otherwise fall back to transport timeout.
    let timeout = data
        .sandbox
        .max_time_ms
        .map(Duration::from_millis)
        .unwrap_or(ctx.transport.timeout);

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(wait_result) => wait_result.context("failed waiting for bash process")?,
        Err(_) => {
            // Timeout: kill the entire process group.
            if let Ok(pgid) = i32::try_from(pid) {
                unsafe { libc::killpg(pgid, libc::SIGKILL) };
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            bail!("bash script timed out after {timeout:?}");
        }
    };

    let stdout_bytes = stdout_reader
        .await
        .context("failed joining stdout reader task")??;
    let stderr_bytes = stderr_reader
        .await
        .context("failed joining stderr reader task")??;

    let exit_code = status
        .code()
        .map(|c| c.clamp(0, u16::MAX as i32) as u16)
        .unwrap_or(1);

    let output_bytes = if stdout_bytes.is_empty() {
        &stderr_bytes
    } else {
        &stdout_bytes
    };

    Ok(RawExecutionResult {
        status: exit_code,
        url: "bash://script".into(),
        body: output_bytes.to_vec(),
        content_type: None,
    })
}

use earl_core::{ProtocolExecutor, StreamingProtocolExecutor};
use tokio::sync::mpsc;

/// Bash protocol executor.
pub struct BashExecutor;

impl ProtocolExecutor for BashExecutor {
    type PreparedData = PreparedBashScript;

    async fn execute(
        &mut self,
        data: &PreparedBashScript,
        ctx: &ExecutionContext,
    ) -> anyhow::Result<RawExecutionResult> {
        execute_bash_once(data, ctx).await
    }
}

/// Streaming bash protocol executor.
///
/// Reuses the same sandboxed process setup as [`BashExecutor`] but streams
/// stdout line-by-line through an `mpsc::Sender<StreamChunk>` instead of
/// buffering the entire output.
pub struct BashStreamExecutor;

impl StreamingProtocolExecutor for BashStreamExecutor {
    type PreparedData = PreparedBashScript;

    async fn execute_stream(
        &mut self,
        data: &PreparedBashScript,
        ctx: &ExecutionContext,
        sender: mpsc::Sender<StreamChunk>,
    ) -> anyhow::Result<StreamMeta> {
        if !sandbox_available() {
            bail!(
                "bash sandbox tool ({}) is not available on this system; \
                 install it or disable the bash feature",
                sandbox_tool_name()
            );
        }

        let mut command =
            build_sandboxed_command(&data.script, &data.env, data.cwd.as_deref(), &data.sandbox)?;

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        if data.stdin.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        let max_memory = data.sandbox.max_memory_bytes;
        let max_cpu_secs = data.sandbox.max_cpu_time_ms.map(|ms| (ms + 999) / 1000);

        // SAFETY: setsid() creates a new session / process group so that we
        // can kill the entire group on timeout without leaking children.
        // setrlimit() calls are safe in a post-fork, pre-exec context.
        unsafe {
            command.pre_exec(move || {
                libc::setsid();
                if let Some(bytes) = max_memory {
                    let rlim = libc::rlimit {
                        rlim_cur: bytes as libc::rlim_t,
                        rlim_max: bytes as libc::rlim_t,
                    };
                    if libc::setrlimit(libc::RLIMIT_AS, &rlim) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                if let Some(secs) = max_cpu_secs {
                    // Note: RLIMIT_CPU grace period sends SIGXCPU then SIGKILL ~1s later.
                    let rlim = libc::rlimit {
                        rlim_cur: secs as libc::rlim_t,
                        rlim_max: secs as libc::rlim_t,
                    };
                    if libc::setrlimit(libc::RLIMIT_CPU, &rlim) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }

        let mut child = command
            .spawn()
            .context("failed spawning sandboxed bash command")?;

        let pid = child
            .id()
            .ok_or_else(|| anyhow::anyhow!("failed to obtain PID of spawned bash process"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed capturing bash stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed capturing bash stderr"))?;

        // Use sandbox output limit if set, otherwise fall back to transport limit.
        let max_bytes = data
            .sandbox
            .max_output_bytes
            .unwrap_or(ctx.transport.max_response_bytes);

        // Drain stderr in a background task so the child process does not block.
        let stderr_task =
            tokio::spawn(async move { read_stream_limited(stderr, max_bytes, "stderr").await });

        // Write stdin if present, then drop the handle so the child sees EOF.
        if let Some(input) = &data.stdin
            && let Some(mut stdin_handle) = child.stdin.take()
        {
            stdin_handle
                .write_all(input.as_bytes())
                .await
                .context("failed writing stdin to bash process")?;
        }

        // Use sandbox timeout if set, otherwise fall back to transport timeout.
        let timeout = data
            .sandbox
            .max_time_ms
            .map(Duration::from_millis)
            .unwrap_or(ctx.transport.timeout);

        // Helper closure to kill the process group.
        let kill_process = |pid: u32| {
            if let Ok(pgid) = i32::try_from(pid) {
                unsafe { libc::killpg(pgid, libc::SIGKILL) };
            }
        };

        // Wrap the entire streaming operation (stdout reading + wait) in a
        // single timeout so that a long-running process cannot stream forever.
        let stream_result = tokio::time::timeout(timeout, async {
            let mut reader = BufReader::new(stdout);
            let mut buf = Vec::new();
            let mut total_bytes: usize = 0;

            loop {
                buf.clear();
                let bytes_read = reader
                    .read_until(b'\n', &mut buf)
                    .await
                    .context("failed reading bash stdout")?;
                if bytes_read == 0 {
                    break;
                }

                total_bytes = total_bytes.saturating_add(bytes_read);
                if total_bytes > max_bytes {
                    // Kill the process and reap it before bailing.
                    kill_process(pid);
                    let _ = child.wait().await;
                    bail!("bash stdout exceeded configured max output bytes ({max_bytes} bytes)");
                }

                let line = String::from_utf8_lossy(&buf).into_owned();
                let chunk = StreamChunk {
                    data: line.into_bytes(),
                    content_type: None,
                };
                if sender.send(chunk).await.is_err() {
                    // Receiver dropped — kill the process and stop.
                    kill_process(pid);
                    break;
                }
            }

            // Drop the sender so the consumer sees the end of the stream.
            drop(sender);

            // Wait for the child to finish.
            let status = child
                .wait()
                .await
                .context("failed waiting for bash process")?;
            Ok::<_, anyhow::Error>(status)
        })
        .await;

        // Abort the stderr task — we don't need it anymore.
        stderr_task.abort();

        match stream_result {
            Ok(Ok(status)) => {
                let exit_code = status
                    .code()
                    .map(|c| c.clamp(0, u16::MAX as i32) as u16)
                    .unwrap_or(1);
                Ok(StreamMeta {
                    status: exit_code,
                    url: "bash://script".into(),
                })
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout: kill the entire process group.
                kill_process(pid);
                let _ = child.kill().await;
                let _ = child.wait().await;
                bail!("bash script timed out after {timeout:?}");
            }
        }
    }
}

async fn read_stream_limited<R>(mut reader: R, limit: usize, label: &str) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut out = Vec::new();
    let mut buf = [0_u8; 8192];

    loop {
        let bytes_read = reader
            .read(&mut buf)
            .await
            .with_context(|| format!("failed reading bash {label}"))?;
        if bytes_read == 0 {
            break;
        }
        if out.len().saturating_add(bytes_read) > limit {
            bail!("bash {label} exceeded configured max_response_bytes ({limit} bytes)");
        }
        out.extend_from_slice(&buf[..bytes_read]);
    }

    Ok(out)
}
