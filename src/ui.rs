use crate::app::{App, HookStatus, InputMode, COMMIT_TYPES};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Top-level: two rows — main content + footer
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.area());

    // Main content: two columns — left (staged + textarea) | right (hook log)
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    // Left column: staged files on top, commit textarea below
    let staged_height = if app.staged_files.is_empty() {
        0
    } else {
        (app.staged_files.len() as u16 + 2).min(8)
    };

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(staged_height), Constraint::Min(3)])
        .split(columns[0]);

    if staged_height > 0 {
        draw_staged_files(f, app, left_chunks[0]);
    }
    draw_textarea(f, app, left_chunks[1]);

    // Right column: hook panel fills the full height
    draw_hook_panel(f, app, columns[1]);

    // Footer
    draw_footer(f, app, rows[1]);
}

fn draw_textarea(f: &mut Frame, app: &mut App, area: Rect) {
    match app.input_mode {
        InputMode::SelectType => draw_type_selector(f, app, area),
        InputMode::EnterScope => draw_scope_input(f, app, area),
        InputMode::EditMessage => {
            let block = Block::default()
                .title(" Commit message ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));

            app.textarea.set_block(block);
            app.textarea.set_cursor_line_style(Style::default());
            f.render_widget(&app.textarea, area);
        }
    }
}

fn draw_type_selector(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = COMMIT_TYPES
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let style = if i == app.type_selection {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {name:<10}"), style),
                Span::styled(
                    format!(" {desc}"),
                    if i == app.type_selection {
                        style
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(" Select commit type (↑/↓ Enter) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_scope_input(f: &mut Frame, app: &App, area: Rect) {
    let (type_name, _) = COMMIT_TYPES[app.type_selection];
    let block = Block::default()
        .title(format!(
            " {type_name} — enter scope (optional, Enter to skip) "
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{}(", type_name),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            &app.scope_input,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("): ", Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, area);
}

fn draw_staged_files(f: &mut Frame, app: &App, area: Rect) {
    let lines: Vec<Line> = app
        .staged_files
        .iter()
        .map(|entry| {
            let color = if entry.starts_with('A') {
                Color::Green
            } else if entry.starts_with('D') {
                Color::Red
            } else if entry.starts_with('M') {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            Line::from(Span::styled(
                format!("  {entry}"),
                Style::default().fg(color),
            ))
        })
        .collect();

    let block = Block::default()
        .title(format!(" Staged files ({}) ", app.staged_files.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn hook_output_lines(app: &App, max_lines: usize, color: Color) -> Vec<Line<'static>> {
    let output = &app.hook_output;
    if output.is_empty() || max_lines == 0 {
        return vec![];
    }

    let end = (app.hook_scroll + 1).min(output.len());
    let start = end.saturating_sub(max_lines);

    let mut lines = Vec::new();

    if start > 0 {
        lines.push(Line::from(Span::styled(
            format!("  ↑ {} more", start),
            Style::default().fg(Color::DarkGray),
        )));
    }

    for line in &output[start..end] {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            Style::default().fg(color),
        )));
    }

    if end < output.len() {
        lines.push(Line::from(Span::styled(
            format!("  ↓ {} more", output.len() - end),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}

fn draw_hook_panel(f: &mut Frame, app: &App, area: Rect) {
    // Inner height = area height - 2 (borders), minus 1 for status line
    let max_output_lines = (area.height as usize).saturating_sub(3);

    let (title, content) = match &app.hook_status {
        HookStatus::NoHook => (
            " Pre-commit hook ",
            vec![Line::from(Span::styled(
                "  No pre-commit hook found — will commit directly",
                Style::default().fg(Color::DarkGray),
            ))],
        ),
        HookStatus::Running => {
            let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frame = (app.tick_count / 2) % spinner.len();
            let mut lines = vec![Line::from(Span::styled(
                format!("  {} Running...", spinner[frame]),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))];
            lines.extend(hook_output_lines(app, max_output_lines, Color::DarkGray));
            (" Pre-commit hook ", lines)
        }
        HookStatus::Passed(elapsed) => (
            " Pre-commit hook ",
            vec![Line::from(Span::styled(
                format!("  ✅ Passed ({elapsed:.1}s)"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ))],
        ),
        HookStatus::Failed(elapsed) => {
            let mut lines = vec![Line::from(Span::styled(
                format!("  ❌ Failed ({elapsed:.1}s)"),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))];
            lines.extend(hook_output_lines(app, max_output_lines, Color::Red));
            (" Pre-commit hook ", lines)
        }
        HookStatus::Waiting => (
            " Pre-commit hook ",
            vec![Line::from(Span::styled(
                "  ⏳ Waiting for hook to finish...",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))],
        ),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(match &app.hook_status {
            HookStatus::Passed(_) => Style::default().fg(Color::Green),
            HookStatus::Failed(_) => Style::default().fg(Color::Red),
            _ => Style::default().fg(Color::Yellow),
        });

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    let spans = match app.input_mode {
        InputMode::SelectType => vec![
            Span::styled("  ↑/↓", key_style),
            Span::raw(": navigate   "),
            Span::styled("Enter", key_style),
            Span::raw(": select   "),
            Span::styled("Esc", key_style),
            Span::raw(": abort"),
        ],
        InputMode::EnterScope => vec![
            Span::styled("  Enter", key_style),
            Span::raw(": confirm   "),
            Span::styled("Esc", key_style),
            Span::raw(": abort"),
        ],
        InputMode::EditMessage => {
            let mut s = vec![
                Span::styled("  Ctrl+S", key_style),
                Span::raw(": commit   "),
                Span::styled("Ctrl+C", key_style),
                Span::raw("/"),
                Span::styled("Esc", key_style),
                Span::raw(": abort"),
            ];
            if matches!(app.hook_status, HookStatus::Failed(_)) {
                s.push(Span::raw("   "));
                s.push(Span::styled("Ctrl+R", key_style));
                s.push(Span::raw(": retry"));
            }
            s
        }
    };

    let footer = Paragraph::new(Line::from(spans));
    f.render_widget(footer, area);
}
