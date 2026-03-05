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
    // Color palette from the logo HTML
    let gold = Color::Rgb(232, 184, 109);
    let green = Color::Rgb(62, 140, 110);
    let cream = Color::Rgb(240, 236, 227);
    let dim_purple = Color::Rgb(90, 90, 110);
    let dark = Color::Rgb(42, 42, 56);

    let s_gold = Style::default().fg(gold);
    let s_gold_bold = Style::default().fg(gold).add_modifier(Modifier::BOLD);
    let s_cream_bold = Style::default().fg(cream).add_modifier(Modifier::BOLD);
    let s_green = Style::default().fg(green);
    let s_dim = Style::default().fg(dim_purple);
    let s_dark = Style::default().fg(dark);

    // Geometric airplane using half-block characters
    // ▀▄█▐▌ give us 2x vertical resolution
    let plane_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("          ·  · ·", s_dark),
            Span::styled("▄▄", s_gold),
        ]),
        Line::from(vec![
            Span::styled("       · ·", s_dark),
            Span::styled("▄▄▄██████████▀▀", s_gold),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("<", s_green),
            Span::styled(" ▄▄████", s_gold),
            Span::styled("▀▀▀▀▀▀▀▀▀", s_gold_bold),
            Span::styled("▀▀▀▀▀  ▀▀", s_gold),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("/>", s_green),
            Span::styled(" ▀▀████", s_gold),
            Span::styled("▄▄▄▄▄▄▄▄▄", s_gold_bold),
            Span::styled("▄▄▄▄▄  ▄▄", s_gold),
        ]),
        Line::from(vec![
            Span::styled("       · ·", s_dark),
            Span::styled("▀▀▀██████████▄▄", s_gold),
        ]),
        Line::from(vec![
            Span::styled("          ·  · ·", s_dark),
            Span::styled("▀▀", s_gold),
        ]),
    ];

    // Center everything vertically
    let content_height = 18; // total lines of content
    let v_pad = area.height.saturating_sub(content_height) / 3; // upper third

    let mut lines: Vec<Line> = Vec::new();

    // Vertical padding
    for _ in 0..v_pad {
        lines.push(Line::from(""));
    }

    // Badge
    lines.push(Line::from(Span::styled(
        "           // indie build · v0.1.0",
        s_dim,
    )));
    lines.push(Line::from(""));

    // Airplane
    for pl in plane_lines {
        lines.push(pl);
    }
    lines.push(Line::from(""));

    // Title
    lines.push(Line::from(vec![
        Span::styled("           A I R P L A N E", s_cream_bold),
    ]));
    lines.push(Line::from(vec![
        Span::styled("           C O D E R", s_gold_bold),
    ]));

    // Divider
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("           ", Style::default()),
        Span::styled("─", s_dark),
        Span::styled("──", Style::default().fg(dim_purple)),
        Span::styled("────", s_gold),
        Span::styled("──", Style::default().fg(dim_purple)),
        Span::styled("─", s_dark),
    ]));

    // Tagline
    lines.push(Line::from(vec![
        Span::styled("           ", Style::default()),
        Span::styled("/* ", s_green),
        Span::styled("ship code from anywhere", s_dim),
        Span::styled(" */", s_green),
    ]));
    lines.push(Line::from(""));

    // Help hints
    lines.push(Line::from(Span::styled(
        "           type anything to start",
        s_cream_bold,
    )));
    lines.push(Line::from(Span::styled(
        "           /model · /help · esc",
        s_dim,
    )));

    let paragraph = Paragraph::new(lines);
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
                    Style::default().fg(Color::Rgb(200, 160, 90)),
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
                .fg(Color::Rgb(232, 184, 109))
                .add_modifier(Modifier::DIM),
        )));
    }

    // Pre-wrap lines at character boundaries so our row count is exact.
    // (Ratatui's word-wrap can use more rows than a simple char-count estimate,
    // causing the auto-scroll to undershoot and cut off bottom content.)
    let lines = wrap_lines_to_width(lines, width);
    let total_visual_rows = lines.len();

    let visible_height = area.height as usize;

    // Scroll: show the bottom unless user scrolled up
    let max_scroll = total_visual_rows.saturating_sub(visible_height);
    // Clamp scroll_offset to valid range
    let clamped_offset = app.scroll_offset.min(max_scroll);
    let scroll = max_scroll.saturating_sub(clamped_offset);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().style(Style::default()))
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
                    Color::Rgb(160, 130, 70)
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
    let gold = Color::Rgb(232, 184, 109);
    let muted_gold = Color::Rgb(200, 160, 90);

    let line = if app.is_processing {
        let elapsed = app.turn_start
            .map(|s| format!("{:.1}s", s.elapsed().as_secs_f64()))
            .unwrap_or_default();
        let tool_info = app.last_tool
            .as_deref()
            .map(|t| format!(" | {t}"))
            .unwrap_or_default();
        Line::from(vec![
            Span::styled(" ● ", Style::default().fg(gold).add_modifier(Modifier::SLOW_BLINK)),
            Span::styled(
                format!("iter {}/20{} | {}", app.iteration_count, tool_info, elapsed),
                Style::default().fg(muted_gold),
            ),
        ])
    } else if !app.context_line.is_empty() {
        Line::from(Span::styled(
            format!(" {}", app.context_line),
            Style::default().fg(muted_gold),
        ))
    } else {
        Line::from(Span::styled(" Ready", Style::default().fg(muted_gold)))
    };

    let paragraph = Paragraph::new(line);

    f.render_widget(paragraph, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let cwd = std::env::current_dir()
        .map(|p| dirs_home(&p))
        .unwrap_or_else(|_| "?".to_string());

    let bar_fg = Color::Rgb(180, 150, 100);
    let fg = Style::default().fg(bar_fg);

    let left = format!(" {} | {}", app.model, cwd);

    // Network indicator: green <3s, yellow 3-10s, red >10s
    let (dot, dot_color, label) = match app.last_latency_ms {
        Some(ms) if ms < 3000 => ("●", Color::Green, format!("{:.1}s", ms as f64 / 1000.0)),
        Some(ms) if ms < 10000 => ("●", Color::Rgb(232, 184, 109), format!("{:.1}s", ms as f64 / 1000.0)),
        Some(ms) => ("●", Color::Red, format!("{:.1}s", ms as f64 / 1000.0)),
        None => ("○", Color::Rgb(60, 55, 45), "—".to_string()),
    };

    let right = format!("{} {} ", label, dot);
    let pad = (area.width as usize).saturating_sub(left.len() + right.len() - dot.len() + 1);

    let line = Line::from(vec![
        Span::styled(left, fg),
        Span::styled(" ".repeat(pad), Style::default()),
        Span::styled(label + " ", Style::default().fg(Color::Rgb(90, 80, 60))),
        Span::styled(dot, Style::default().fg(dot_color)),
        Span::styled(" ", Style::default()),
    ]);

    let paragraph = Paragraph::new(line);

    f.render_widget(paragraph, area);
}

/// Split lines that exceed `width` into multiple lines, preserving styles.
/// Each output line fits within the terminal width, giving an exact row count.
fn wrap_lines_to_width(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let char_count: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if char_count <= width {
            out.push(line);
        } else if line.spans.len() == 1 {
            // Fast path: single span — split at character boundaries
            let span = &line.spans[0];
            let style = span.style;
            let chars: Vec<char> = span.content.chars().collect();
            for chunk in chars.chunks(width) {
                let s: String = chunk.iter().collect();
                out.push(Line::from(Span::styled(s, style)));
            }
        } else {
            // Multi-span: collect all chars with styles, then chunk
            let styled_chars: Vec<(char, Style)> = line
                .spans
                .iter()
                .flat_map(|s| s.content.chars().map(move |c| (c, s.style)))
                .collect();
            for chunk in styled_chars.chunks(width) {
                let spans: Vec<Span> = chunk_to_spans(chunk);
                out.push(Line::from(spans));
            }
        }
    }
    out
}

/// Group consecutive chars with the same style into Spans.
fn chunk_to_spans(chars: &[(char, Style)]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut cur_style = chars[0].1;
    for &(c, style) in chars {
        if style == cur_style {
            buf.push(c);
        } else {
            spans.push(Span::styled(std::mem::take(&mut buf), cur_style));
            cur_style = style;
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, cur_style));
    }
    spans
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
