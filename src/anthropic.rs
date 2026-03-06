use anyhow::{Context, Result};
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
enum AuthMethod {
    ApiKey(String),
    OAuth(String),
}

#[derive(Clone)]
pub struct AnthropicClient {
    auth: AuthMethod,
    client: reqwest::Client,
}

// --- Anthropic API types ---

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
}

impl AnthropicClient {
    pub fn new() -> Result<Self> {
        // Try OAuth token first (sk-ant-oat01-...), then API key
        let auth = if let Some(token) = load_env_file_key("ANTHROPIC_AUTH_TOKEN")
            .or_else(|| std::env::var("ANTHROPIC_AUTH_TOKEN").ok())
        {
            AuthMethod::OAuth(token)
        } else if let Some(key) = load_env_file_key("ANTHROPIC_API_KEY")
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        {
            AuthMethod::ApiKey(key)
        } else {
            anyhow::bail!(
                "No Anthropic credentials found. Set ANTHROPIC_AUTH_TOKEN (OAuth) or ANTHROPIC_API_KEY in .env or environment."
            );
        };

        Ok(Self {
            auth,
            client: reqwest::Client::new(),
        })
    }

    /// Chat with Anthropic, returning our standard types
    pub async fn chat(
        &self,
        model: &str,
        messages: &[Message],
        tools: Option<&[ToolDef]>,
        max_tokens: Option<u32>,
    ) -> Result<ChatResponse> {
        // Extract system prompt from messages
        let system = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        // Convert our messages to Anthropic format
        let anthropic_messages = convert_messages(messages);

        // Convert tool definitions
        let anthropic_tools = tools.map(|t| {
            t.iter()
                .map(|td| AnthropicTool {
                    name: td.function.name.clone(),
                    description: td.function.description.clone(),
                    input_schema: td.function.parameters.clone(),
                })
                .collect()
        });

        let request = AnthropicRequest {
            model: model.to_string(),
            max_tokens: max_tokens.unwrap_or(16384),
            system,
            messages: anthropic_messages,
            tools: anthropic_tools,
        };

        let mut req = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");

        req = match &self.auth {
            AuthMethod::ApiKey(key) => req.header("x-api-key", key),
            AuthMethod::OAuth(token) => req
                .header("Authorization", format!("Bearer {token}"))
                .header("anthropic-beta", "oauth-2025-04-20"),
        };

        let resp = req
            .json(&request)
            .send()
            .await
            .context("Failed to connect to Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API returned {status}: {body}");
        }

        let anthropic_resp: AnthropicResponse = resp
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        // Convert back to our standard types
        Ok(convert_response(anthropic_resp))
    }
}

/// Convert our internal messages to Anthropic format
fn convert_messages(messages: &[Message]) -> Vec<AnthropicMessage> {
    let mut result: Vec<AnthropicMessage> = Vec::new();

    for msg in messages {
        if msg.role == "system" {
            continue; // system is handled separately
        }

        if msg.role == "tool" {
            // Tool results become tool_result blocks in the last user message
            let tool_use_id = msg.tool_call_id.clone()
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| format!("tool_{}", result.len()));
            let block = ContentBlock::ToolResult {
                tool_use_id,
                content: msg.content.clone(),
            };

            // Append to existing user message with blocks, or create new one
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    match &mut last.content {
                        AnthropicContent::Blocks(blocks) => {
                            blocks.push(block);
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            result.push(AnthropicMessage {
                role: "user".to_string(),
                content: AnthropicContent::Blocks(vec![block]),
            });
            continue;
        }

        if msg.role == "assistant" {
            let mut blocks: Vec<ContentBlock> = Vec::new();

            if !msg.content.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: msg.content.clone(),
                });
            }

            // Convert tool_calls to tool_use blocks
            if let Some(tool_calls) = &msg.tool_calls {
                for tc in tool_calls {
                    blocks.push(ContentBlock::ToolUse {
                        id: tc.id.clone().unwrap_or_else(|| tc.function.name.clone()),
                        name: tc.function.name.clone(),
                        input: tc.function.arguments.clone(),
                    });
                }
            }

            if blocks.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: String::new(),
                });
            }

            result.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: AnthropicContent::Blocks(blocks),
            });
            continue;
        }

        // User message
        result.push(AnthropicMessage {
            role: "user".to_string(),
            content: AnthropicContent::Text(msg.content.clone()),
        });
    }

    result
}

/// Convert Anthropic response back to our standard types
fn convert_response(resp: AnthropicResponse) -> ChatResponse {
    let mut text_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for block in resp.content {
        match block {
            ContentBlock::Text { text } => {
                if !text.is_empty() {
                    text_parts.push(text);
                }
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id: Some(id),
                    call_type: Some("function".to_string()),
                    function: ToolCallFunction {
                        name,
                        arguments: input,
                    },
                });
            }
            ContentBlock::ToolResult { .. } => {} // shouldn't appear in response
        }
    }

    ChatResponse {
        message: Message {
            role: "assistant".to_string(),
            content: text_parts.join("\n"),
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
        },
        done: resp.stop_reason.as_deref() == Some("end_turn"),
        total_duration: None,
        eval_count: None,
    }
}

/// Load a key from .env files, checking multiple locations:
/// 1. ~/.airplane/.env  (persistent user config)
/// 2. ./.env            (current working directory, for dev convenience)
fn load_env_file_key(key: &str) -> Option<String> {
    let paths = [
        std::env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".airplane").join(".env")),
        std::env::current_dir().ok().map(|d| d.join(".env")),
    ];
    for path in paths.into_iter().flatten() {
        if let Some(val) = read_env_key(&path, key) {
            return Some(val);
        }
    }
    None
}

fn read_env_key(path: &std::path::Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(value) = rest.strip_prefix('=') {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

pub fn is_anthropic_model(model: &str) -> bool {
    model.starts_with("claude-") || model == "sonnet-fast"
}
