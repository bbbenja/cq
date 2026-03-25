use anyhow::{bail, Result};
use std::path::PathBuf;
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

/// Read the commit template from git config, if set.
/// Strips comment lines (starting with `#`).
pub fn commit_template() -> Option<String> {
    let output = Command::new("git")
        .args(["config", "commit.template"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw_path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if raw_path.is_empty() {
        return None;
    }

    // Resolve ~ to home directory
    let path = if let Some(rest) = raw_path.strip_prefix("~/") {
        if let Some(home) = dirs_path() {
            home.join(rest)
        } else {
            PathBuf::from(&raw_path)
        }
    } else {
        PathBuf::from(&raw_path)
    };

    let content = std::fs::read_to_string(path).ok()?;
    let filtered: Vec<&str> = content.lines().filter(|l| !l.starts_with('#')).collect();
    let result = filtered.join("\n").trim().to_string();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn dirs_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// Return the message of the last commit (for --amend).
pub fn last_commit_message() -> Option<String> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%B"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let msg = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if msg.is_empty() {
        None
    } else {
        Some(msg)
    }
}

/// Return the last N commits as short one-line summaries.
pub fn recent_commits(n: usize) -> Vec<String> {
    let output = Command::new("git")
        .args(["log", &format!("-{n}"), "--format=%h %s"])
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.to_string())
            .collect(),
        _ => vec![],
    }
}

/// Return the list of staged files with their status (e.g. "M  src/main.rs").
pub fn staged_files() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-status"])
        .output()?;

    if !output.status.success() {
        bail!("Failed to list staged files");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|l| l.to_string()).collect())
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
