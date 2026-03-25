use crate::app::{App, HookStatus};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App) {
    let staged_height = if app.staged_files.is_empty() {
        0
    } else {
        (app.staged_files.len() as u16 + 2).min(8)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),                // commit message textarea
            Constraint::Length(staged_height), // staged files panel
            Constraint::Length(8),             // hook status panel
            Constraint::Length(1),             // footer
        ])
        .split(f.area());

    draw_textarea(f, app, chunks[0]);
    if staged_height > 0 {
        draw_staged_files(f, app, chunks[1]);
    }
    draw_hook_panel(f, app, chunks[2]);
    draw_footer(f, chunks[3]);
}

fn draw_textarea(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Commit message ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    app.textarea.set_block(block);
    app.textarea.set_cursor_line_style(Style::default());
    f.render_widget(&app.textarea, area);
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

    // Window ends at hook_scroll + 1 (inclusive), capped to output length
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

fn draw_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "  Ctrl+S",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": commit   "),
        Span::styled(
            "Ctrl+C",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("/"),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": abort"),
    ]));
    f.render_widget(footer, area);
}
