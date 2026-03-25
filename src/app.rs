use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::Duration;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

use crate::git;
use crate::hook::{self, HookEvent};
use crate::ui;
use crate::CommitOpts;

#[derive(Debug, Clone)]
pub enum HookStatus {
    NoHook,
    Running,
    Passed(f64),
    Failed(f64),
    /// User pressed submit while hook is still running
    Waiting,
}

pub struct App<'a> {
    pub textarea: TextArea<'a>,
    pub hook_status: HookStatus,
    pub hook_output: Vec<String>,
    pub tick_count: usize,
    /// Set when user wants to submit but hook is still running
    pub pending_submit: bool,
    pub commit_opts: CommitOpts,
}

impl<'a> App<'a> {
    fn new(commit_opts: CommitOpts) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Enter commit message...");
        Self {
            textarea,
            hook_status: HookStatus::Running,
            hook_output: Vec::new(),
            tick_count: 0,
            pending_submit: false,
            commit_opts,
        }
    }
}

pub async fn run(opts: CommitOpts) -> Result<()> {
    // Pre-flight checks
    git::ensure_in_repo()?;

    if !opts.skip_staged_check() && !git::has_staged_changes()? {
        anyhow::bail!("Nothing to commit (no staged changes). Use `git add` first.");
    }

    // Check for pre-commit hook
    let hook_path = hook::find_pre_commit_hook();

    // Set up the terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, hook_path, opts).await;

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    match result {
        Ok(Some(msg)) => {
            println!("{msg}");
            Ok(())
        }
        Ok(None) => {
            println!("Commit aborted.");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Run the TUI event loop. Returns Ok(Some(message)) on successful commit,
/// Ok(None) on abort, or Err on failure.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    hook_path: Option<std::path::PathBuf>,
    opts: CommitOpts,
) -> Result<Option<String>> {
    let mut app = App::new(opts);

    // Set up hook channel
    let (tx, mut rx) = mpsc::unbounded_channel::<HookEvent>();

    // Spawn hook if it exists
    let _hook_handle = if let Some(path) = hook_path {
        Some(hook::spawn_hook(path, tx)?)
    } else {
        app.hook_status = HookStatus::NoHook;
        None
    };

    loop {
        // Draw
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Process hook events (non-blocking drain)
        while let Ok(evt) = rx.try_recv() {
            match evt {
                HookEvent::Output(line) => {
                    app.hook_output.push(line);
                }
                HookEvent::Finished {
                    success,
                    elapsed_secs,
                } => {
                    if success {
                        app.hook_status = HookStatus::Passed(elapsed_secs);
                        // If user was waiting to submit, do it now
                        if app.pending_submit {
                            return do_commit(&app);
                        }
                    } else {
                        app.hook_status = HookStatus::Failed(elapsed_secs);
                        app.pending_submit = false;
                    }
                }
            }
        }

        // Poll for keyboard events with a short timeout (for spinner animation)
        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                match handle_key(&mut app, key) {
                    Action::Continue => {}
                    Action::Abort => return Ok(None),
                    Action::Submit => {
                        let message = app.textarea.lines().join("\n").trim().to_string();
                        if message.is_empty() {
                            continue;
                        }
                        match &app.hook_status {
                            HookStatus::Passed(_) | HookStatus::NoHook => {
                                return do_commit(&app);
                            }
                            HookStatus::Failed(_) => {
                                // Can't commit with failed hook
                                continue;
                            }
                            HookStatus::Running | HookStatus::Waiting => {
                                app.pending_submit = true;
                                app.hook_status = HookStatus::Waiting;
                            }
                        }
                    }
                }
            }
        }

        app.tick_count += 1;
    }
}

enum Action {
    Continue,
    Abort,
    Submit,
}

fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    match key {
        // Ctrl+C or Esc → abort
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }
        | KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            ..
        } => Action::Abort,

        // Ctrl+S → submit
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Submit,

        // Ctrl+Enter → submit (crossterm sends Enter with NONE or CONTROL depending on terminal)
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Submit,

        // Everything else goes to the textarea
        _ => {
            app.textarea.input(key);
            Action::Continue
        }
    }
}

fn do_commit(app: &App) -> Result<Option<String>> {
    let message = app.textarea.lines().join("\n").trim().to_string();
    if message.is_empty() {
        return Ok(None);
    }
    let output = git::commit(&message, &app.commit_opts.extra_args)?;
    Ok(Some(output))
}
