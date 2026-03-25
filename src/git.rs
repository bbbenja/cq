use anyhow::{bail, Result};
use std::process::Command;

/// Check we are inside a git repository.
pub fn ensure_in_repo() -> Result<()> {
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        bail!("Not inside a git repository");
    }
    Ok(())
}

/// Check that there are staged changes to commit.
pub fn has_staged_changes() -> Result<bool> {
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()?;

    // exit code 1 means there ARE differences (i.e. staged changes exist)
    Ok(!status.success())
}

/// Finalize the commit with --no-verify (hooks already ran manually).
/// `extra_args` are appended to the command (e.g. --amend, --signoff).
pub fn commit(message: &str, extra_args: &[String]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(["commit", "--no-verify", "-m", message]);
    cmd.args(extra_args);
    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git commit failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.to_string())
}
