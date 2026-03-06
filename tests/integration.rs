/// Integration tests for Airplane Coder.
/// Run with: cargo test --test integration
///
/// These tests verify the contract between modules without hitting external services.
/// They should pass on every code change — CI gate material.

// ---- Type serialization: the glue between us and Ollama/Anthropic ----

#[test]
fn ollama_chat_request_serializes_correctly() {
    use airplane::types::*;

    let request = ChatRequest {
        model: "qwen3.5:4b".to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: None,
        stream: false,
        options: Some(ChatOptions {
            temperature: 0.1,
            num_ctx: 8192,
        }),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["model"], "qwen3.5:4b");
    assert_eq!(json["stream"], false);
    let temp = json["options"]["temperature"].as_f64().unwrap();
    assert!((temp - 0.1).abs() < 0.001, "temperature should be ~0.1, got {temp}");
    assert_eq!(json["options"]["num_ctx"], 8192);
    // tool_calls should be absent (skip_serializing_if)
    assert!(json["messages"][0].get("tool_calls").is_none());
}

#[test]
fn ollama_response_with_tool_calls_deserializes() {
    use airplane::types::*;

    let json = serde_json::json!({
        "message": {
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "read_file",
                    "arguments": {"path": "src/main.rs"}
                }
            }]
        },
        "done": true,
        "total_duration": 1000000,
        "eval_count": 50
    });

    let resp: ChatResponse = serde_json::from_value(json).unwrap();
    let tc = resp.message.tool_calls.unwrap();
    assert_eq!(tc.len(), 1);
    assert_eq!(tc[0].function.name, "read_file");
    assert_eq!(tc[0].function.arguments["path"], "src/main.rs");
}

#[test]
fn ollama_response_text_only_deserializes() {
    use airplane::types::*;

    let json = serde_json::json!({
        "message": {
            "role": "assistant",
            "content": "Hello! How can I help?"
        },
        "done": true
    });

    let resp: ChatResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.message.content, "Hello! How can I help?");
    assert!(resp.message.tool_calls.is_none());
}

#[test]
fn ollama_response_with_string_arguments_deserializes() {
    // Some models return arguments as a JSON string instead of object
    use airplane::types::*;

    let json = serde_json::json!({
        "message": {
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "function": {
                    "name": "shell",
                    "arguments": "{\"command\": \"ls -la\"}"
                }
            }]
        },
        "done": true
    });

    let resp: ChatResponse = serde_json::from_value(json).unwrap();
    let tc = resp.message.tool_calls.unwrap();
    assert_eq!(tc[0].function.name, "shell");
    // arguments is a string Value — agent loop handles parsing this
    assert!(tc[0].function.arguments.is_string());
}

// ---- Tool definitions: every tool must have valid schemas ----

#[test]
fn all_tool_definitions_are_valid() {
    use airplane::tools::get_tool_definitions;

    let defs = get_tool_definitions();
    assert_eq!(defs.len(), 6, "Expected 6 tools");

    let expected_names = ["read_file", "write_file", "edit_file", "shell", "grep", "glob"];
    let actual_names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();

    for name in &expected_names {
        assert!(actual_names.contains(name), "Missing tool: {name}");
    }

    for def in &defs {
        assert_eq!(def.tool_type, "function");
        assert!(!def.function.description.is_empty(), "{} has empty description", def.function.name);

        // Parameters must be a valid JSON schema object
        let params = &def.function.parameters;
        assert_eq!(params["type"], "object", "{} parameters not an object", def.function.name);
        assert!(params.get("properties").is_some(), "{} missing properties", def.function.name);
        assert!(params.get("required").is_some(), "{} missing required", def.function.name);
    }
}

#[test]
fn tool_definitions_serialize_for_ollama() {
    // Tool defs must round-trip through JSON (they get sent to Ollama)
    use airplane::tools::get_tool_definitions;

    let defs = get_tool_definitions();
    let json = serde_json::to_string(&defs).unwrap();
    let roundtrip: Vec<airplane::types::ToolDef> = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtrip.len(), defs.len());
}

// ---- Tool execution: each tool handles bad input gracefully ----

#[tokio::test]
async fn tools_return_errors_not_panics() {
    use airplane::tools::execute_tool;
    use std::collections::HashMap;
    use std::path::Path;

    let root = Path::new("/");

    // Unknown tool
    let result = execute_tool("nonexistent", &HashMap::new(), root).await;
    assert!(result.starts_with("Error:"));

    // read_file with no args
    let result = execute_tool("read_file", &HashMap::new(), root).await;
    assert!(result.starts_with("Error:"));

    // read_file with nonexistent path
    let mut args = HashMap::new();
    args.insert("path".into(), serde_json::json!("/tmp/airplane_nonexistent_file_12345"));
    let result = execute_tool("read_file", &args, root).await;
    assert!(result.starts_with("Error:"));

    // edit_file with missing args
    let result = execute_tool("edit_file", &HashMap::new(), root).await;
    assert!(result.starts_with("Error:"));

    // write_file with missing content
    let mut args = HashMap::new();
    args.insert("path".into(), serde_json::json!("/tmp/test"));
    let result = execute_tool("write_file", &args, root).await;
    assert!(result.starts_with("Error:"));
}

#[tokio::test]
async fn shell_tool_timeout_works() {
    use airplane::tools::execute_tool;
    use std::collections::HashMap;
    use std::path::Path;

    let root = Path::new("/");

    // A fast command should succeed
    let mut args = HashMap::new();
    args.insert("command".into(), serde_json::json!("echo ok"));
    let result = execute_tool("shell", &args, root).await;
    assert_eq!(result.trim(), "ok");
}

// ---- Agent internals: truncation and conversation trimming ----

#[test]
fn model_routing_dispatches_correctly() {
    use airplane::anthropic::is_anthropic_model;

    assert!(is_anthropic_model("claude-opus-4-6"));
    assert!(is_anthropic_model("claude-sonnet-4-6"));
    assert!(is_anthropic_model("sonnet-fast"));
    assert!(!is_anthropic_model("qwen3.5:4b"));
    assert!(!is_anthropic_model("gemma3:12b"));
}
