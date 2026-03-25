mod app;
mod git;
mod hook;
mod install;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cq", version, about = "Commit quick — type your message while hooks run")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up the global git alias so `git commit` calls cq
    Install,
    /// Remove the global git alias
    Uninstall,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Install) => install::install()?,
        Some(Commands::Uninstall) => install::uninstall()?,
        None => app::run().await?,
    }

    Ok(())
}
