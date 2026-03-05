use anyhow::Result;
use crate::anthropic::{self, AnthropicClient};
use crate::ollama::OllamaClient;
use crate::tools;
use crate::types::*;
use std::collections::HashMap;

const MAX_ITERATIONS: usize = 20;
const MAX_TOOL_RESULT_CHARS: usize = 2000;
const MAX_CONTEXT_MESSAGES: usize = 40; // keep last N messages when trimming

/// Holds both LLM backends
#[derive(Clone)]
pub struct LlmBackend {
    pub ollama: OllamaClient,
    pub anthropic: Option<AnthropicClient>,
}

impl LlmBackend {
    pub fn new() -> Self {
        Self {
            ollama: OllamaClient::new(),
            anthropic: AnthropicClient::new().ok(),
        }
    }

    async fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[ToolDef]>,
    ) -> Result<ChatResponse> {
        if anthropic::is_anthropic_model(model) {
            let client = self
                .anthropic
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY not set. Add it to .env or environment."))?;
            client.chat(model, messages, tools).await
        } else {
            self.ollama.chat(model, messages, tools).await
        }
    }
}

fn system_prompt() -> String {
    let mut prompt = String::from(
        "You are Airplane Coder, a coding assistant.\n\
         You help users with software engineering tasks: reading code, writing code, debugging, refactoring, running tests, and more.\n\n\
         Guidelines:\n\
         - Read files before modifying them\n\
         - Use edit_file for targeted changes, write_file for new files\n\
         - Run tests after making changes when appropriate\n\
         - Be concise — lead with actions, not explanations\n\
         - When searching code, use grep for content and glob for file paths\n",
    );

    // Load AIRPLANE.md from cwd if it exists
    if let Ok(cwd) = std::env::current_dir() {
        let airplane_md = cwd.join("AIRPLANE.md");
        if let Ok(content) = std::fs::read_to_string(&airplane_md) {
            prompt.push_str("\n--- Project Instructions (AIRPLANE.md) ---\n");
            prompt.push_str(&content);
            prompt.push('\n');
        }
    }

    prompt
}

fn truncate_tool_result(result: &str) -> String {
    if result.len() <= MAX_TOOL_RESULT_CHARS {
        return result.to_string();
    }
    let half = MAX_TOOL_RESULT_CHARS / 2;
    let start = &result[..half];
    let end = &result[result.len() - half..];
    let truncated = result.len() - MAX_TOOL_RESULT_CHARS;
    format!("{start}\n\n... {truncated} chars truncated ...\n\n{end}")
}

fn trim_conversation(messages: &mut Vec<Message>) {
    // Keep system message (index 0) + last MAX_CONTEXT_MESSAGES
    if messages.len() <= MAX_CONTEXT_MESSAGES + 1 {
        return;
    }
    let keep_from = messages.len() - MAX_CONTEXT_MESSAGES;
    let system = messages[0].clone();
    let kept: Vec<Message> = messages[keep_from..].to_vec();
    messages.clear();
    messages.push(system);
    messages.extend(kept);
}

/// Events sent from agent to TUI
#[derive(Debug, Clone)]
pub enum AgentEvent {
    AssistantText(String),
    ToolCall(String),       // formatted description
    ToolResult(String),     // tool output
    Done,
    Error(String),
    MessagesSync(Vec<Message>), // sync agent messages back to TUI
}

pub async fn run_agent_turn(
    backend: &LlmBackend,
    model: &str,
    messages: &mut Vec<Message>,
    event_tx: &tokio::sync::mpsc::UnboundedSender<AgentEvent>,
) -> Result<()> {
    // Ensure system message is first
    if messages.is_empty() || messages[0].role != "system" {
        messages.insert(
            0,
            Message {
                role: "system".to_string(),
                content: system_prompt(),
                tool_calls: None,
                tool_call_id: None,
            },
        );
    }

    let tool_defs = tools::get_tool_definitions();

    for _iteration in 0..MAX_ITERATIONS {
        trim_conversation(messages);

        let response = backend.chat(model, messages, Some(&tool_defs)).await?;
        let msg = response.message;

        // If there's text content, send it
        if !msg.content.is_empty() {
            let _ = event_tx.send(AgentEvent::AssistantText(msg.content.clone()));
        }

        // Add assistant message to history
        messages.push(msg.clone());

        // Check for tool calls
        let tool_calls = match &msg.tool_calls {
            Some(tc) if !tc.is_empty() => tc.clone(),
            _ => {
                // No tool calls — turn is done
                let _ = event_tx.send(AgentEvent::Done);
                return Ok(());
            }
        };

        // Execute each tool call
        for tc in &tool_calls {
            let name = &tc.function.name;
            let args: HashMap<String, serde_json::Value> = match &tc.function.arguments {
                serde_json::Value::Object(map) => {
                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                }
                serde_json::Value::String(s) => {
                    // Sometimes models return arguments as a JSON string
                    serde_json::from_str(s).unwrap_or_default()
                }
                _ => HashMap::new(),
            };

            // Format tool call for display
            let display = format_tool_call(name, &args);
            let _ = event_tx.send(AgentEvent::ToolCall(display));

            // Execute
            let result = tools::execute_tool(name, &args).await;
            let truncated = truncate_tool_result(&result);

            let _ = event_tx.send(AgentEvent::ToolResult(truncated.clone()));

            // Add tool result to conversation
            let call_id = tc.id.clone().unwrap_or_else(|| name.clone());
            messages.push(Message {
                role: "tool".to_string(),
                content: truncated,
                tool_calls: None,
                tool_call_id: Some(call_id),
            });
        }

        // Loop continues — model needs to respond to tool results
    }

    let _ = event_tx.send(AgentEvent::Error(
        "Reached maximum iteration limit (20)".to_string(),
    ));
    let _ = event_tx.send(AgentEvent::Done);
    Ok(())
}

fn format_tool_call(name: &str, args: &HashMap<String, serde_json::Value>) -> String {
    let get = |key: &str| -> String {
        args.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    match name {
        "read_file" => format!("read {}", get("path")),
        "write_file" => format!("write {}", get("path")),
        "edit_file" => format!("edit {}", get("path")),
        "shell" => format!("$ {}", get("command")),
        "grep" => {
            let path = get("path");
            let path = if path.is_empty() { ".".to_string() } else { path };
            format!("grep \"{}\" in {}", get("pattern"), path)
        }
        "glob" => format!("glob {}", get("pattern")),
        _ => format!("{name}(...)"),
    }
}

/// Simplified version for REPL mode (no TUI)
pub async fn run_agent_turn_repl(
    backend: &LlmBackend,
    model: &str,
    messages: &mut Vec<Message>,
) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    run_agent_turn(backend, model, messages, &tx).await?;
    drop(tx);

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::AssistantText(text) => println!("\n{text}"),
            AgentEvent::ToolCall(desc) => println!("  > {desc}"),
            AgentEvent::ToolResult(result) => {
                let preview: String = result.lines().take(5).collect::<Vec<_>>().join("\n");
                println!("    {preview}");
            }
            AgentEvent::Done => {}
            AgentEvent::Error(e) => eprintln!("Error: {e}"),
            AgentEvent::MessagesSync(_) => {} // only used by TUI
        }
    }
    Ok(())
}
