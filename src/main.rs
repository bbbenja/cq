mod app;
mod git;
mod hook;
mod install;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cq",
    version,
    about = "Commit quick — type your message while hooks run"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Amend the previous commit
    #[arg(long)]
    amend: bool,

    /// Add Signed-off-by trailer
    #[arg(short = 's', long)]
    signoff: bool,

    /// Stage all modified/deleted files before committing
    #[arg(short = 'a', long)]
    all: bool,

    /// Override the commit author
    #[arg(long, value_name = "AUTHOR")]
    author: Option<String>,

    /// Override the author date
    #[arg(long, value_name = "DATE")]
    date: Option<String>,

    /// Allow empty commits (no staged changes required)
    #[arg(long)]
    allow_empty: bool,

    /// Use conventional commit format (type/scope selector)
    #[arg(short = 'c', long)]
    conventional: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up the global git alias so `git commit` calls cq
    Install,
    /// Remove the global git alias
    Uninstall,
}

/// Options forwarded to the final `git commit` call.
#[derive(Clone, Default)]
pub struct CommitOpts {
    pub amend: bool,
    pub all: bool,
    pub allow_empty: bool,
    pub conventional: bool,
    pub extra_args: Vec<String>,
}

impl CommitOpts {
    fn from_cli(cli: &Cli) -> Self {
        let mut extra_args = Vec::new();
        if cli.amend {
            extra_args.push("--amend".into());
        }
        if cli.signoff {
            extra_args.push("--signoff".into());
        }
        if cli.all {
            extra_args.push("--all".into());
        }
        if let Some(ref author) = cli.author {
            extra_args.push(format!("--author={author}"));
        }
        if let Some(ref date) = cli.date {
            extra_args.push(format!("--date={date}"));
        }
        if cli.allow_empty {
            extra_args.push("--allow-empty".into());
        }
        Self {
            amend: cli.amend,
            all: cli.all,
            allow_empty: cli.allow_empty,
            conventional: cli.conventional,
            extra_args,
        }
    }

    /// Whether we can skip the "must have staged changes" check.
    pub fn skip_staged_check(&self) -> bool {
        self.amend || self.all || self.allow_empty
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Install) => install::install()?,
        Some(Commands::Uninstall) => install::uninstall()?,
        None => {
            let opts = CommitOpts::from_cli(&cli);
            app::run(opts).await?;
        }
    }

    Ok(())
}
