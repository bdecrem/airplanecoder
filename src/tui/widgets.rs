use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{App, UiMessage};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // messages
            Constraint::Length(3), // input
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    if app.show_splash {
        render_splash(f, chunks[0]);
    } else {
        render_messages(f, chunks[0], app);
    }
    render_input(f, chunks[1], app);
    render_status_bar(f, chunks[2], app);
}

fn render_splash(f: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default().fg(Color::Cyan);
    let bold_cyan = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let bold_white = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
    let gray = Style::default().fg(Color::Gray);

    let splash = vec![
        Line::from(""),
        Line::from(Span::styled(r"     /\  ", cyan)),
        Line::from(Span::styled(r"    /  | ", cyan)),
        Line::from(Span::styled(r"   / /| |", cyan)),
        Line::from(Span::styled(r"  /_/_|_/", cyan)),
        Line::from(""),
        Line::from(Span::styled("  AIRPLANE CODER", bold_cyan)),
        Line::from(""),
        Line::from(Span::styled(
            "  your code. your machine. no cloud required.",
            gray,
        )),
        Line::from(""),
        Line::from(Span::styled("  type anything to start hacking", bold_white)),
        Line::from(Span::styled(
            "  /model to switch brains   /help for more   esc to bail",
            dim,
        )),
    ];

    let paragraph = Paragraph::new(splash);
    f.render_widget(paragraph, area);
}

fn render_messages(f: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        match msg {
            UiMessage::User(text) => {
                lines.push(Line::from(Span::styled(
                    format!("> {text}"),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            UiMessage::Assistant(text) => {
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(Color::White),
                    )));
                }
            }
            UiMessage::ToolCall(desc) => {
                lines.push(Line::from(Span::styled(
                    format!("  > {desc}"),
                    Style::default().fg(Color::Cyan),
                )));
            }
            UiMessage::ToolResult(result) => {
                for line in result.lines().take(10) {
                    lines.push(Line::from(Span::styled(
                        format!("    {line}"),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            UiMessage::System(text) => {
                lines.push(Line::from(Span::styled(
                    format!("  {text}"),
                    Style::default().fg(Color::Yellow),
                )));
            }
            UiMessage::Info(text) => {
                lines.push(Line::from(Span::styled(
                    format!("  {text}"),
                    Style::default().fg(Color::White),
                )));
            }
        }
        lines.push(Line::from(""));
    }

    if app.is_processing {
        lines.push(Line::from(Span::styled(
            "  thinking...",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::DIM),
        )));
    }

    let total_lines = lines.len() as u16;
    let visible_height = area.height;

    // Calculate scroll: we want to show the bottom unless user scrolled up
    let max_scroll = total_lines.saturating_sub(visible_height) as usize;
    let scroll = max_scroll.saturating_sub(app.scroll_offset);

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let input_text = if app.is_processing {
        "processing...".to_string()
    } else {
        app.input.clone()
    };

    let paragraph = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.is_processing {
                    Color::DarkGray
                } else {
                    Color::Cyan
                })),
        )
        .style(Style::default().fg(if app.is_processing {
            Color::DarkGray
        } else {
            Color::White
        }));

    f.render_widget(paragraph, area);

    // Place cursor
    if !app.is_processing {
        f.set_cursor_position((
            area.x + 1 + app.cursor_pos as u16,
            area.y + 1,
        ));
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let cwd = std::env::current_dir()
        .map(|p| {
            let home = dirs_home(&p);
            home
        })
        .unwrap_or_else(|_| "?".to_string());

    let status = format!(" {} | {}", app.model, cwd);

    let paragraph = Paragraph::new(Line::from(Span::styled(
        status,
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray),
    )))
    .style(Style::default().bg(Color::DarkGray));

    f.render_widget(paragraph, area);
}

fn dirs_home(path: &std::path::Path) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = std::path::Path::new(&home);
        if let Ok(stripped) = path.strip_prefix(home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.display().to_string()
}
