use anyhow::{bail, Result};
use std::process::Command;

/// Install the global git alias: git commit → cq
pub fn install() -> Result<()> {
    // Find the cq binary path
    let which = Command::new("which").arg("cq").output()?;
    if !which.status.success() {
        bail!("cq binary not found in PATH. Install it first (cargo install --path .)");
    }
    let bin_path = String::from_utf8(which.stdout)?.trim().to_string();

    let alias_value = format!("!{bin_path}");

    let status = Command::new("git")
        .args(["config", "--global", "alias.commit", &alias_value])
        .status()?;

    if !status.success() {
        bail!("Failed to set global git alias");
    }

    println!("Installed: git commit → cq");
    println!("Alias set to: {alias_value}");
    Ok(())
}

/// Remove the global git alias
pub fn uninstall() -> Result<()> {
    let status = Command::new("git")
        .args(["config", "--global", "--unset", "alias.commit"])
        .status()?;

    if !status.success() {
        // --unset returns 5 if the key doesn't exist
        println!("No global commit alias was set (nothing to remove).");
    } else {
        println!("Removed global git commit alias.");
    }

    Ok(())
}
