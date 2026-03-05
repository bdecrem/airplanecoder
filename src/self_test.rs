use anyhow::Result;
use airplane::ollama::OllamaClient;
use airplane::tools;
use std::collections::HashMap;

pub async fn run_self_test() -> Result<()> {
    println!("Airplane Coder — Self Test\n");

    // 1. Check Ollama connectivity
    print!("  Ollama connectivity... ");
    let client = OllamaClient::new();
    if client.is_available().await {
        println!("OK");
    } else {
        println!("FAIL");
        eprintln!("    Ollama is not running. Start it with: ollama serve");
        std::process::exit(1);
    }

    // 2. List models
    print!("  List models... ");
    match client.list_models().await {
        Ok(models) => {
            println!("OK ({} models)", models.len());
            for m in &models {
                println!("    - {m}");
            }
        }
        Err(e) => {
            println!("FAIL: {e}");
        }
    }

    // 3. Parse sample tool call
    print!("  Tool call parsing... ");
    let sample = r#"{"id":"1","type":"function","function":{"name":"read_file","arguments":{"path":"Cargo.toml"}}}"#;
    match serde_json::from_str::<airplane::types::ToolCall>(sample) {
        Ok(tc) => println!("OK (parsed {} call)", tc.function.name),
        Err(e) => println!("FAIL: {e}"),
    }

    // 4. Tool definitions
    print!("  Tool definitions... ");
    let defs = tools::get_tool_definitions();
    println!("OK ({} tools)", defs.len());
    for d in &defs {
        println!("    - {}", d.function.name);
    }

    // 5. Read own Cargo.toml
    print!("  read_file(Cargo.toml)... ");
    let mut args = HashMap::new();
    args.insert("path".into(), serde_json::json!("Cargo.toml"));
    args.insert("limit".into(), serde_json::json!(5));
    let result = tools::execute_tool("read_file", &args).await;
    if result.contains("[package]") {
        println!("OK");
    } else {
        println!("FAIL");
        eprintln!("    Got: {}", &result[..result.len().min(100)]);
    }

    // 6. Shell echo
    print!("  shell(echo hello)... ");
    let mut args = HashMap::new();
    args.insert("command".into(), serde_json::json!("echo hello"));
    let result = tools::execute_tool("shell", &args).await;
    if result.trim() == "hello" {
        println!("OK");
    } else {
        println!("FAIL: {result}");
    }

    // 7. Glob own source
    print!("  glob(src/**/*.rs)... ");
    let mut args = HashMap::new();
    args.insert("pattern".into(), serde_json::json!("src/**/*.rs"));
    let result = tools::execute_tool("glob", &args).await;
    if result.contains("main.rs") {
        println!("OK");
    } else {
        println!("FAIL: {result}");
    }

    println!("\nSelf-test complete.");
    Ok(())
}
