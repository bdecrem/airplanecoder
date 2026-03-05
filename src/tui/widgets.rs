use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{App, UiMessage};

pub fn render(f: &mut Frame, app: &App) {
    // Calculate input height based on wrapped lines (1-3 visible text lines)
    let inner_width = f.area().width.saturating_sub(2) as usize; // minus borders
    let input_lines = if inner_width == 0 { 1 } else {
        let text_len = if app.is_processing { 13 } else { app.input.len() }; // "processing..." = 13
        ((text_len.max(1) - 1) / inner_width.max(1)) + 1
    };
    let visible_text_lines = input_lines.min(3) as u16;
    let input_height = visible_text_lines + 2; // + borders

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),              // messages
            Constraint::Length(input_height), // input (3-5)
            Constraint::Length(1),            // context bar
            Constraint::Length(1),            // status bar
        ])
        .split(f.area());

    if app.show_splash {
        render_splash(f, chunks[0]);
    } else {
        render_messages(f, chunks[0], app);
    }
    render_input(f, chunks[1], app, input_lines, visible_text_lines as usize);
    render_context_bar(f, chunks[2], app);
    render_status_bar(f, chunks[3], app);
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
    let width = area.width as usize;

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

    // Count actual visual rows after wrapping
    let total_visual_rows: usize = lines.iter().map(|line| {
        let line_len: usize = line.spans.iter().map(|s| s.content.len()).sum();
        if width == 0 { 1 } else { ((line_len.max(1) - 1) / width) + 1 }
    }).sum();

    let visible_height = area.height as usize;

    // Scroll: show the bottom unless user scrolled up
    let max_scroll = total_visual_rows.saturating_sub(visible_height);
    let scroll = max_scroll.saturating_sub(app.scroll_offset);

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn render_input(f: &mut Frame, area: Rect, app: &App, total_lines: usize, visible_lines: usize) {
    let input_text = if app.is_processing {
        "processing...".to_string()
    } else {
        app.input.clone()
    };

    let inner_width = area.width.saturating_sub(2) as usize;

    // Calculate which line the cursor is on and its column
    let cursor_line = if inner_width > 0 { app.cursor_pos / inner_width } else { 0 };
    let cursor_col = if inner_width > 0 { app.cursor_pos % inner_width } else { 0 };

    // Scroll: keep cursor line visible within the visible area
    let scroll_row = if total_lines <= visible_lines {
        0
    } else {
        // Ensure cursor line is in view
        let max_scroll = total_lines.saturating_sub(visible_lines);
        cursor_line.saturating_sub(visible_lines - 1).min(max_scroll)
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
        }))
        .wrap(Wrap { trim: false })
        .scroll((scroll_row as u16, 0));

    f.render_widget(paragraph, area);

    // Place cursor
    if !app.is_processing {
        let visible_cursor_line = cursor_line.saturating_sub(scroll_row) as u16;
        f.set_cursor_position((
            area.x + 1 + cursor_col as u16,
            area.y + 1 + visible_cursor_line,
        ));
    }
}

fn render_context_bar(f: &mut Frame, area: Rect, app: &App) {
    let content = if app.is_processing {
        let elapsed = app.turn_start
            .map(|s| format!("{:.1}s", s.elapsed().as_secs_f64()))
            .unwrap_or_default();
        let tool_info = app.last_tool
            .as_deref()
            .map(|t| format!(" | last: {t}"))
            .unwrap_or_default();
        format!(
            " ⏳ working… iter {}/20{} | {}",
            app.iteration_count, tool_info, elapsed
        )
    } else if !app.context_line.is_empty() {
        format!(" {}", app.context_line)
    } else {
        " Ready".to_string()
    };

    let paragraph = Paragraph::new(Line::from(Span::styled(
        content,
        Style::default()
            .fg(Color::Cyan)
            .bg(Color::Black),
    )))
    .style(Style::default().bg(Color::Black));

    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let cwd = std::env::current_dir()
        .map(|p| dirs_home(&p))
        .unwrap_or_else(|_| "?".to_string());

    let bg = Style::default().fg(Color::White).bg(Color::DarkGray);

    let left = format!(" {} | {}", app.model, cwd);

    // Network indicator: green <3s, yellow 3-10s, red >10s
    let (dot, dot_color, label) = match app.last_latency_ms {
        Some(ms) if ms < 3000 => ("●", Color::Green, format!("{:.1}s", ms as f64 / 1000.0)),
        Some(ms) if ms < 10000 => ("●", Color::Yellow, format!("{:.1}s", ms as f64 / 1000.0)),
        Some(ms) => ("●", Color::Red, format!("{:.1}s", ms as f64 / 1000.0)),
        None => ("○", Color::DarkGray, "—".to_string()),
    };

    let right = format!("{} {} ", label, dot);
    let pad = (area.width as usize).saturating_sub(left.len() + right.len() - dot.len() + 1);

    let line = Line::from(vec![
        Span::styled(left, bg),
        Span::styled(" ".repeat(pad), Style::default().bg(Color::DarkGray)),
        Span::styled(label + " ", Style::default().fg(Color::DarkGray).bg(Color::DarkGray)),
        Span::styled(dot, Style::default().fg(dot_color).bg(Color::DarkGray)),
        Span::styled(" ", Style::default().bg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(line)
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
