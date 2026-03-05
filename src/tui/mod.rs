pub mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, EnableBracketedPaste, DisableBracketedPaste, EnableMouseCapture, DisableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use tokio::sync::mpsc;

use airplane::agent::{self, AgentEvent, LlmBackend};
use airplane::anthropic;
use airplane::settings::Settings;
use airplane::types::*;

#[derive(Debug, Clone)]
pub enum UiMessage {
    User(String),
    Assistant(String),
    ToolCall(String),
    ToolResult(String),
    System(String),
    Info(String),
}

pub struct App {
    pub messages: Vec<UiMessage>,
    pub input: String,
    pub cursor_pos: usize,
    pub model: String,
    pub scroll_offset: usize,
    pub is_processing: bool,
    pub should_quit: bool,
    pub agent_messages: Vec<Message>,
    pub show_splash: bool,
    pub context_line: String,
    pub iteration_count: usize,
    pub last_tool: Option<String>,
    pub turn_start: Option<std::time::Instant>,
    pub settings: Settings,
    pub last_latency_ms: Option<u64>,
    pub user_scrolled: bool,
}

impl App {
    pub fn new(model: String, settings: Settings) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            model,
            scroll_offset: 0,
            is_processing: false,
            should_quit: false,
            agent_messages: Vec::new(),
            show_splash: true,
            context_line: String::new(),
            iteration_count: 0,
            last_tool: None,
            turn_start: None,
            settings,
            last_latency_ms: None,
            user_scrolled: false,
        }
    }

    fn handle_slash_command(&mut self, input: &str) -> Option<SlashAction> {
        let parts: Vec<&str> = input.trim().splitn(2, ' ').collect();
        match parts[0] {
            "/exit" | "/quit" => {
                self.should_quit = true;
                None
            }
            "/clear" => {
                self.messages.clear();
                self.agent_messages.clear();
                self.scroll_offset = 0;
                self.messages.push(UiMessage::System("Conversation cleared.".into()));
                None
            }
            "/model" => {
                if parts.len() > 1 {
                    let new_model = parts[1].to_string();
                    self.model = new_model.clone();
                    self.messages.push(UiMessage::System(format!("Switched to model: {new_model}")));
                } else {
                    self.messages.push(UiMessage::Info(format!("Current model: {}", self.model)));
                    self.messages.push(UiMessage::Info(
                        "Local: qwen3.5:0.8b, qwen3.5:2b, qwen3.5:4b, qwen3.5:9b, gemma3:12b".into(),
                    ));
                    self.messages.push(UiMessage::Info(
                        "Cloud: claude-opus-4-6, claude-sonnet-4-6, sonnet-fast".into(),
                    ));
                }
                None
            }
            "/settings" => {
                use airplane::settings::ResumeModel;
                if parts.len() > 1 {
                    match parts[1].trim() {
                        "resume last" | "resume last-used" => {
                            self.settings.resume_model = ResumeModel::LastUsed;
                            self.settings.save();
                            self.messages.push(UiMessage::System(
                                "Resume: will start with last-used model.".into(),
                            ));
                        }
                        "resume default" => {
                            self.settings.resume_model = ResumeModel::Default;
                            self.settings.save();
                            self.messages.push(UiMessage::System(
                                "Resume: will start with default model.".into(),
                            ));
                        }
                        _ => {
                            self.messages.push(UiMessage::System(
                                "Unknown setting. Try: /settings resume last  or  /settings resume default".into(),
                            ));
                        }
                    }
                } else {
                    self.messages.push(UiMessage::Info("Settings:".into()));
                    self.messages.push(UiMessage::Info(format!(
                        "  resume: {}",
                        self.settings.resume_model
                    )));
                    self.messages.push(UiMessage::Info("".into()));
                    self.messages.push(UiMessage::Info(
                        "Change with: /settings resume last  or  /settings resume default".into(),
                    ));
                }
                None
            }
            "/help" => {
                self.messages.push(UiMessage::Info("Commands:".into()));
                self.messages.push(UiMessage::Info("  /model [name]  — show or switch model".into()));
                self.messages.push(UiMessage::Info("  /settings      — view or change settings".into()));
                self.messages.push(UiMessage::Info("  /clear         — reset conversation".into()));
                self.messages.push(UiMessage::Info("  /help          — show this help".into()));
                self.messages.push(UiMessage::Info("  /exit          — quit".into()));
                self.messages.push(UiMessage::Info("".into()));
                self.messages.push(UiMessage::Info("Scrolling: mouse wheel, PageUp/PageDown, Shift+Up/Down".into()));
                None
            }
            _ => {
                self.messages.push(UiMessage::System(format!("Unknown command: {}", parts[0])));
                None
            }
        }
    }
}

enum SlashAction {}

pub async fn run_tui() -> Result<()> {
    let settings = Settings::load();
    let model = settings.startup_model();
    let mut app = App::new(model.clone(), settings);

    let backend = LlmBackend::new();

    // Check connectivity for the default model
    if anthropic::is_anthropic_model(&model) {
        if backend.anthropic.is_none() {
            app.messages.push(UiMessage::System(
                "Warning: ANTHROPIC_API_KEY not set. Add it to .env or environment.".into(),
            ));
        }
    } else if !backend.ollama.is_available().await {
        app.messages.push(UiMessage::System(
            "Warning: Ollama is not running. Start it with: ollama serve".into(),
        ));
    }

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableBracketedPaste)?;
    stdout().execute(EnableMouseCapture)?;
    let term_backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(term_backend)?;
    terminal.clear()?;

    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel::<AgentEvent>();

    let result = run_event_loop(&mut terminal, &mut app, &backend, &agent_tx, &mut agent_rx).await;

    // Save last-used model
    app.settings.last_model = Some(app.model.clone());
    app.settings.save();

    // Restore terminal
    stdout().execute(DisableMouseCapture)?;
    stdout().execute(DisableBracketedPaste)?;
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    backend: &LlmBackend,
    agent_tx: &mpsc::UnboundedSender<AgentEvent>,
    agent_rx: &mut mpsc::UnboundedReceiver<AgentEvent>,
) -> Result<()> {
    let mut agent_handle: Option<tokio::task::JoinHandle<()>> = None;
    let mut last_esc: Option<std::time::Instant> = None;

    loop {
        // Draw
        terminal.draw(|f| widgets::render(f, app))?;

        // Poll for crossterm events (50ms timeout)
        if event::poll(std::time::Duration::from_millis(50))? {
            let ev = event::read()?;

            // Handle pasted text (bracketed paste mode)
            if let Event::Paste(text) = &ev {
                if !app.is_processing {
                    // Replace newlines with spaces, insert at cursor
                    let clean: String = text.chars().map(|c| if c == '\n' || c == '\r' { ' ' } else { c }).collect();
                    app.input.insert_str(app.cursor_pos, &clean);
                    app.cursor_pos += clean.len();
                    app.show_splash = false;
                }
            }

            if let Event::Mouse(mouse) = &ev {
                match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        app.scroll_offset = app.scroll_offset.saturating_add(3);
                        app.user_scrolled = app.scroll_offset > 0;
                    }
                    MouseEventKind::ScrollDown => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(3);
                        app.user_scrolled = app.scroll_offset > 0;
                    }
                    _ => {}
                }
            } else if let Event::Key(key) = ev {
                if app.show_splash {
                    app.show_splash = false;
                    continue;
                }

                match key {
                    // Ctrl+C: cancel agent or quit
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        ..
                    } => {
                        if app.is_processing {
                            if let Some(handle) = agent_handle.take() {
                                handle.abort();
                            }
                            app.is_processing = false;
                            app.messages.push(UiMessage::System("Cancelled.".into()));
                        } else {
                            app.should_quit = true;
                        }
                    }
                    // Esc: cancel agent (single tap), quit (double tap when idle)
                    KeyEvent {
                        code: KeyCode::Esc,
                        ..
                    } => {
                        if app.is_processing {
                            // Single Esc cancels the agent
                            if let Some(handle) = agent_handle.take() {
                                handle.abort();
                            }
                            app.is_processing = false;
                            app.messages.push(UiMessage::System("Cancelled.".into()));
                        } else {
                            // Double-tap Esc to quit when idle (within 500ms)
                            let now = std::time::Instant::now();
                            if let Some(prev) = last_esc {
                                if now.duration_since(prev).as_millis() < 500 {
                                    app.should_quit = true;
                                }
                            }
                            last_esc = Some(now);
                        }
                    }
                    // Scroll: Shift+Up
                    KeyEvent {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::SHIFT,
                        ..
                    } => {
                        app.scroll_offset = app.scroll_offset.saturating_add(3);
                        app.user_scrolled = app.scroll_offset > 0;
                    }
                    // Scroll: Shift+Down
                    KeyEvent {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::SHIFT,
                        ..
                    } => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(3);
                        app.user_scrolled = app.scroll_offset > 0;
                    }
                    // PageUp
                    KeyEvent {
                        code: KeyCode::PageUp,
                        ..
                    } => {
                        let page = terminal.size()?.height.saturating_sub(4) as usize;
                        app.scroll_offset = app.scroll_offset.saturating_add(page);
                        app.user_scrolled = app.scroll_offset > 0;
                    }
                    // PageDown
                    KeyEvent {
                        code: KeyCode::PageDown,
                        ..
                    } => {
                        let page = terminal.size()?.height.saturating_sub(4) as usize;
                        app.scroll_offset = app.scroll_offset.saturating_sub(page);
                        app.user_scrolled = app.scroll_offset > 0;
                    }
                    // Enter: submit input
                    KeyEvent {
                        code: KeyCode::Enter,
                        ..
                    } => {
                        if !app.input.is_empty() && !app.is_processing {
                            let input = app.input.clone();
                            app.input.clear();
                            app.cursor_pos = 0;
                            app.scroll_offset = 0;
                            app.user_scrolled = false;

                            if input.starts_with('/') {
                                app.handle_slash_command(&input);
                            } else {
                                app.show_splash = false;
                                app.messages.push(UiMessage::User(input.clone()));
                                app.is_processing = true;
                                app.iteration_count = 0;
                                app.last_tool = None;
                                app.turn_start = Some(std::time::Instant::now());
                                app.context_line.clear();

                                // Add user message to agent conversation
                                app.agent_messages.push(Message {
                                    role: "user".to_string(),
                                    content: input,
                                    tool_calls: None,
                                    tool_call_id: None,
                                });

                                // Spawn agent task
                                let backend_clone = backend.clone();
                                let model_clone = app.model.clone();
                                let mut messages_clone = app.agent_messages.clone();
                                let tx = agent_tx.clone();

                                agent_handle = Some(tokio::spawn(async move {
                                    let _ = agent::run_agent_turn(
                                        &backend_clone,
                                        &model_clone,
                                        &mut messages_clone,
                                        &tx,
                                    )
                                    .await
                                    .map_err(|e| {
                                        let _ = tx.send(AgentEvent::Error(e.to_string()));
                                        let _ = tx.send(AgentEvent::Done);
                                    });
                                    // Send updated messages back — we'll use a special event
                                    let _ = tx.send(AgentEvent::MessagesSync(messages_clone));
                                }));
                            }
                        }
                    }
                    // Backspace
                    KeyEvent {
                        code: KeyCode::Backspace,
                        ..
                    } => {
                        if app.cursor_pos > 0 {
                            app.cursor_pos -= 1;
                            app.input.remove(app.cursor_pos);
                        }
                    }
                    // Left arrow
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    } => {
                        app.cursor_pos = app.cursor_pos.saturating_sub(1);
                    }
                    // Right arrow
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    } => {
                        if app.cursor_pos < app.input.len() {
                            app.cursor_pos += 1;
                        }
                    }
                    // Home
                    KeyEvent {
                        code: KeyCode::Home,
                        ..
                    } => {
                        app.cursor_pos = 0;
                    }
                    // End
                    KeyEvent {
                        code: KeyCode::End,
                        ..
                    } => {
                        app.cursor_pos = app.input.len();
                    }
                    // Regular character input
                    KeyEvent {
                        code: KeyCode::Char(c),
                        modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
                        ..
                    } => {
                        app.input.insert(app.cursor_pos, c);
                        app.cursor_pos += 1;
                    }
                    _ => {}
                }
            }
        }

        // Drain agent events
        while let Ok(event) = agent_rx.try_recv() {
            match event {
                AgentEvent::AssistantText(text) => {
                    app.messages.push(UiMessage::Assistant(text));
                    if !app.user_scrolled { app.scroll_offset = 0; }
                }
                AgentEvent::ToolCall(desc) => {
                    app.iteration_count += 1;
                    // Extract tool name from desc (format: "tool_name({...})")
                    let tool_name = desc.split('(').next().unwrap_or(&desc).trim().to_string();
                    app.last_tool = Some(tool_name);
                    app.messages.push(UiMessage::ToolCall(desc));
                    if !app.user_scrolled { app.scroll_offset = 0; }
                }
                AgentEvent::ToolResult(result) => {
                    // Truncate for display
                    let display = if result.len() > 500 {
                        format!("{}...", &result[..500])
                    } else {
                        result
                    };
                    app.messages.push(UiMessage::ToolResult(display));
                    if !app.user_scrolled { app.scroll_offset = 0; }
                }
                AgentEvent::Done => {
                    app.is_processing = false;
                    if let Some(start) = app.turn_start.take() {
                        let elapsed = start.elapsed();
                        app.context_line = format!(
                            "Done — {} tool call{} in {:.1}s",
                            app.iteration_count,
                            if app.iteration_count == 1 { "" } else { "s" },
                            elapsed.as_secs_f64()
                        );
                    }
                }
                AgentEvent::Error(e) => {
                    app.messages.push(UiMessage::System(format!("Error: {e}")));
                    app.is_processing = false;
                    app.context_line = format!("Error after {} tool calls", app.iteration_count);
                }
                AgentEvent::Latency(ms) => {
                    app.last_latency_ms = Some(ms);
                }
                AgentEvent::MessagesSync(msgs) => {
                    app.agent_messages = msgs;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
