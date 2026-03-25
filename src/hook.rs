use anyhow::Result;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum HookEvent {
    /// A line of output (stdout or stderr) from the hook
    Output(String),
    /// Hook finished with the given exit success status and elapsed time
    Finished { success: bool, elapsed_secs: f64 },
}

/// Returns the path to .git/hooks/pre-commit if it exists and is executable.
pub fn find_pre_commit_hook() -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8(output.stdout).ok()?.trim().to_string();
    let hook_path = PathBuf::from(root).join(".git/hooks/pre-commit");

    if hook_path.exists() {
        // Check if executable (unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&hook_path).ok()?;
            if meta.permissions().mode() & 0o111 == 0 {
                return None;
            }
        }
        Some(hook_path)
    } else {
        None
    }
}

/// Spawn the pre-commit hook and stream its output through the channel.
/// Returns a JoinHandle; sends HookEvents through `tx`.
pub fn spawn_hook(
    hook_path: PathBuf,
    tx: mpsc::UnboundedSender<HookEvent>,
) -> Result<tokio::task::JoinHandle<()>> {
    let handle = tokio::spawn(async move {
        let start = Instant::now();

        let result = run_hook(&hook_path, &tx).await;

        let elapsed = start.elapsed().as_secs_f64();
        let success = result.unwrap_or(false);

        let _ = tx.send(HookEvent::Finished {
            success,
            elapsed_secs: elapsed,
        });
    });

    Ok(handle)
}

async fn run_hook(hook_path: &PathBuf, tx: &mpsc::UnboundedSender<HookEvent>) -> Result<bool> {
    // Get git toplevel so we run the hook from the repo root
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await?;
    let root = String::from_utf8(output.stdout)?.trim().to_string();

    let mut child = Command::new(hook_path)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let tx2 = tx.clone();

    let stdout_handle = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx2.send(HookEvent::Output(line));
            }
        }
    });

    let tx3 = tx.clone();
    let stderr_handle = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx3.send(HookEvent::Output(line));
            }
        }
    });

    let _ = stdout_handle.await;
    let _ = stderr_handle.await;

    let status = child.wait().await?;
    Ok(status.success())
}
