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

pub const COMMIT_TYPES: &[(&str, &str)] = &[
    ("feat", "A new feature"),
    ("fix", "A bug fix"),
    ("chore", "Maintenance tasks"),
    ("refactor", "Code restructuring"),
    ("docs", "Documentation changes"),
    ("test", "Adding or updating tests"),
    ("style", "Formatting, whitespace"),
    ("ci", "CI/CD changes"),
    ("perf", "Performance improvements"),
    ("build", "Build system changes"),
];

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    SelectType,
    EnterScope,
    EditMessage,
}

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
    pub staged_files: Vec<String>,
    pub hook_scroll: usize,
    pub hook_auto_scroll: bool,
    pub input_mode: InputMode,
    pub type_selection: usize,
    pub scope_input: String,
}

impl<'a> App<'a> {
    fn new(commit_opts: CommitOpts, template: Option<String>, staged_files: Vec<String>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Enter commit message...");
        if let Some(ref tmpl) = template {
            let lines: Vec<&str> = tmpl.lines().collect();
            textarea = TextArea::new(lines.iter().map(|l| l.to_string()).collect());
        }
        let input_mode = if commit_opts.conventional {
            InputMode::SelectType
        } else {
            InputMode::EditMessage
        };
        Self {
            textarea,
            hook_status: HookStatus::Running,
            hook_output: Vec::new(),
            tick_count: 0,
            pending_submit: false,
            commit_opts,
            staged_files,
            hook_scroll: 0,
            hook_auto_scroll: true,
            input_mode,
            type_selection: 0,
            scope_input: String::new(),
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

    // Load commit template and staged files
    let template = git::commit_template();
    let staged_files = git::staged_files().unwrap_or_default();

    let result = run_app(&mut terminal, hook_path, opts, template, staged_files).await;

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
    template: Option<String>,
    staged_files: Vec<String>,
) -> Result<Option<String>> {
    let mut app = App::new(opts, template, staged_files);

    // Set up hook channel
    let (tx, mut rx) = mpsc::unbounded_channel::<HookEvent>();

    // Spawn hook if it exists
    if let Some(ref path) = hook_path {
        hook::spawn_hook(path.clone(), tx)?;
    } else {
        app.hook_status = HookStatus::NoHook;
    }

    loop {
        // Draw
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Process hook events (non-blocking drain)
        while let Ok(evt) = rx.try_recv() {
            match evt {
                HookEvent::Output(line) => {
                    app.hook_output.push(line);
                    if app.hook_auto_scroll {
                        app.hook_scroll = app.hook_output.len().saturating_sub(1);
                    }
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
                    Action::Retry => {
                        if let Some(ref path) = hook_path {
                            app.hook_status = HookStatus::Running;
                            app.hook_output.clear();
                            app.hook_scroll = 0;
                            app.hook_auto_scroll = true;
                            let (new_tx, new_rx) = mpsc::unbounded_channel::<HookEvent>();
                            rx = new_rx;
                            hook::spawn_hook(path.clone(), new_tx)?;
                        }
                    }
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
    Retry,
}

fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    // Global keybindings (all modes)
    match key {
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => return Action::Abort,
        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            ..
        } => return Action::Abort,
        _ => {}
    }

    match app.input_mode {
        InputMode::SelectType => handle_key_select_type(app, key),
        InputMode::EnterScope => handle_key_enter_scope(app, key),
        InputMode::EditMessage => handle_key_edit_message(app, key),
    }
}

fn handle_key_select_type(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Up => {
            if app.type_selection > 0 {
                app.type_selection -= 1;
            }
            Action::Continue
        }
        KeyCode::Down => {
            if app.type_selection < COMMIT_TYPES.len() - 1 {
                app.type_selection += 1;
            }
            Action::Continue
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::EnterScope;
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn handle_key_enter_scope(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => {
            let (type_name, _) = COMMIT_TYPES[app.type_selection];
            let prefix = if app.scope_input.is_empty() {
                format!("{type_name}: ")
            } else {
                format!("{type_name}({}): ", app.scope_input)
            };
            app.textarea.insert_str(prefix);
            app.input_mode = InputMode::EditMessage;
            Action::Continue
        }
        KeyCode::Backspace => {
            app.scope_input.pop();
            Action::Continue
        }
        KeyCode::Char(c) => {
            app.scope_input.push(c);
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn handle_key_edit_message(app: &mut App, key: KeyEvent) -> Action {
    match key {
        // Ctrl+S → submit
        KeyEvent {
            code: KeyCode::Char('s'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Submit,

        // Ctrl+Enter → submit
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::CONTROL,
            ..
        } => Action::Submit,

        // Ctrl+R → retry hook (only when failed)
        KeyEvent {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            if matches!(app.hook_status, HookStatus::Failed(_)) {
                Action::Retry
            } else {
                Action::Continue
            }
        }

        // Alt+Up / Alt+Down → scroll hook output
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            app.hook_scroll = app.hook_scroll.saturating_sub(1);
            app.hook_auto_scroll = false;
            Action::Continue
        }
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::ALT,
            ..
        } => {
            if app.hook_scroll < app.hook_output.len().saturating_sub(1) {
                app.hook_scroll += 1;
            }
            if app.hook_scroll >= app.hook_output.len().saturating_sub(1) {
                app.hook_auto_scroll = true;
            }
            Action::Continue
        }

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
